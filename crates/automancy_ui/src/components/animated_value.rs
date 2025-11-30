use crate::*;

#[derive(Debug, Clone)]
#[must_use = "yakui widgets do nothing if you don't `show` them"]
pub struct AnimatedValue {
    anim: AnimationChannel<f32>,
    pub ease_duration: Duration,
    pub ease_function: EasingFunction,
}

auto_builders!(AnimatedValue {
    ease_duration: Duration,
    ease_function: EasingFunction
});

impl AnimatedValue {
    pub fn new(keys: &[(f32, f32)]) -> Self {
        debug_assert!(keys.is_sorted_by_key(|v| v.0));

        let anim = AnimationChannel::new(
            keys.iter()
                .map(|&(seconds, value)| AnimationFrame {
                    timestamp: Duration::from_secs_f32(seconds),
                    value,
                    in_tangent: None,
                    out_tangent: None,
                })
                .collect(),
            Interpolation::Linear,
            false,
        );

        Self {
            anim,
            ease_duration: Duration::from_secs(1),
            ease_function: EasingFunction::new(easings::Linear),
        }
    }

    pub fn interpolate(mut self, interpolation: Interpolation) -> Self {
        self.anim.interpolation = interpolation;
        self
    }

    pub fn looping(mut self, looping: bool) -> Self {
        self.anim.looping = looping;
        self
    }

    #[track_caller]
    pub fn show(self, f: f32) -> f32 {
        *widget::<AnimatedValueWidget>((self, f))
    }
}

#[derive(Debug)]
pub struct AnimatedValueWidget {
    easing: AnimationChannel<f32>,
}

impl Widget for AnimatedValueWidget {
    type Props<'a> = (AnimatedValue, f32);

    type Response = f32;

    fn new() -> Self {
        Self {
            easing: AnimationChannel::new(
                vec![
                    AnimationFrame {
                        timestamp: Duration::ZERO,
                        value: 0.0,
                        in_tangent: None,
                        out_tangent: None,
                    },
                    AnimationFrame {
                        timestamp: Duration::MAX,
                        value: 0.0,
                        in_tangent: None,
                        out_tangent: None,
                    },
                ],
                Interpolation::Linear,
                false,
            ),
        }
    }

    fn update(&mut self, (props, target_f): Self::Props<'_>) -> Self::Response {
        if props.ease_duration != self.easing.total_duration {
            self.easing.total_duration = props.ease_duration;
            self.easing.keyframes[1].timestamp = props.ease_duration;
        }

        if self.easing.keyframes[1].value != target_f {
            self.easing.keyframes[0].value = self.easing.sample_at_instant(Instant::now()).clamped01();
            self.easing.keyframes[1].value = target_f;

            self.easing.anim_start = Instant::now();
        }

        let f = get_value(&props, &mut self.easing);

        props.anim.sample_at_timestamp(props.anim.total_duration.mul_f32(f))
    }
}

#[inline]
fn get_value(props: &AnimatedValue, easing: &mut AnimationChannel<f32>) -> f32 {
    props.ease_function.ease(easing.sample_at_instant(Instant::now()).clamped01())
}

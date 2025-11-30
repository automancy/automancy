use core::{fmt::Debug, time::Duration};
use std::time::Instant;

use automancy_data::math::Transform;

/// (copied from [`gltf::animation::Interpolation`])
/// Specifies an interpolation algorithm.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Interpolation {
    /// Linear interpolation.
    ///
    /// The animated values are linearly interpolated between keyframes.
    /// When targeting a rotation, spherical linear interpolation (slerp) should be
    /// used to interpolate quaternions. The number output of elements must equal
    /// the number of input elements.
    Linear,

    /// Step interpolation.
    ///
    /// The animated values remain constant to the output of the first keyframe,
    /// until the next keyframe. The number of output elements must equal the number
    /// of input elements.
    Step,

    /// Cubic spline interpolation.
    ///
    /// The animation's interpolation is computed using a cubic spline with specified
    /// tangents. The number of output elements must equal three times the number of
    /// input elements. For each input element, the output stores three elements, an
    /// in-tangent, a spline vertex, and an out-tangent. There must be at least two
    /// keyframes when using this interpolation
    CubicSpline,
}

impl From<gltf::animation::Interpolation> for Interpolation {
    fn from(value: gltf::animation::Interpolation) -> Self {
        match value {
            gltf::animation::Interpolation::Linear => Interpolation::Linear,
            gltf::animation::Interpolation::Step => Interpolation::Step,
            gltf::animation::Interpolation::CubicSpline => Interpolation::CubicSpline,
        }
    }
}

pub trait AnimPrimitive: Copy {
    fn mul(self, v: f32) -> Self;
    fn add(self, v: Self) -> Self;
    fn lerp(a: Self, b: Self, t: f32) -> Self;
}

impl AnimPrimitive for Transform {
    fn mul(self, v: f32) -> Self {
        Transform {
            position: self.position * v,
            orientation: self.orientation * v,
            scale: self.scale * v,
        }
    }

    fn add(self, v: Self) -> Self {
        Transform {
            position: self.position + v.position,
            orientation: self.orientation * v.orientation,
            scale: self.scale * v.scale,
        }
    }

    fn lerp(a: Self, b: Self, t: f32) -> Self {
        vek::Lerp::lerp_unclamped(a, b, t)
    }
}

macro_rules! impl_anim_prim_float {
    ($ty:ty) => {
        impl AnimPrimitive for $ty {
            fn mul(self, v: f32) -> Self {
                self * v as $ty
            }

            fn add(self, v: Self) -> Self {
                self * v
            }

            fn lerp(a: Self, b: Self, t: f32) -> Self {
                vek::Lerp::lerp_unclamped(a, b, t as $ty)
            }
        }
    };
}

impl_anim_prim_float!(f32);
impl_anim_prim_float!(f64);

pub trait AnimatedValue: AnimPrimitive {
    /// Interpolate between `a` and `b` linearly, based on factor `t` between `[0 ~ 1]`.
    fn interpolate_linear(a: Self, b: Self, t: f32) -> Self;
    /// Interpolate between `a` and `b` according to `pt` (previousTangent) and `nt` (nextTangent), forming a Cubic Spline, based on factor `t` between `[0 ~ 1]`.
    fn interpolate_cubic_spline(a: Self, b: Self, pt: Self, nt: Self, t: f32) -> Self;
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl<T: AnimPrimitive> AnimatedValue for T {
    fn interpolate_linear(a: Self, b: Self, t: f32) -> Self {
        T::lerp(a, b, t)
    }

    /**
        Pseudocode from https://github.khronos.org/glTF-Tutorials/gltfTutorial/gltfTutorial_007_Animations.html#cubic-spline-interpolation:

        ```
        Point cubicSpline(previousPoint, previousTangent, nextPoint, nextTangent, interpolationValue)
            t = interpolationValue
            t2 = t * t
            t3 = t2 * t

            return (2 * t3 - 3 * t2 + 1) * previousPoint + (t3 - 2 * t2 + t) * previousTangent + (-2 * t3 + 3 * t2) * nextPoint + (t3 - t2) * nextTangent;
        ```
    */
    fn interpolate_cubic_spline(a: Self, b: Self, pt: Self, nt: Self, t: f32) -> Self {
        let t2 = t * t;
        let t3 = t2 * t;

        let n0 = a.mul(2.0 * t3 - 3.0 * t2 + 1.0);
        let n1 = pt.mul(t3 - 2.0 * t2 + t);
        let n2 = b.mul(-2.0 * t3 + 3.0 * t2);
        let n3 = nt.mul(t3 - t2);

        n0.add(n1).add(n2).add(n3)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimationFrame<Value: AnimatedValue> {
    /// The "time" of the animation from the start of the animation.
    ///
    /// For example, an animation might look like this:
    /// ```
    /// [0.0s, 0.5s, 1.0s, 1.5s, 2.0s]
    /// ```
    pub timestamp: Duration,
    pub value: Value,
    pub in_tangent: Option<Value>,
    pub out_tangent: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnimationChannel<Value: AnimatedValue> {
    pub total_duration: Duration,
    pub keyframes: Vec<AnimationFrame<Value>>,
    pub interpolation: Interpolation,
    pub looping: bool,
    pub anim_start: Instant,
}

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl<Value: AnimatedValue> AnimationChannel<Value> {
    #[inline]
    pub fn new(keyframes: Vec<AnimationFrame<Value>>, interpolation: Interpolation, looping: bool) -> Self {
        Self {
            total_duration: keyframes.last().unwrap().timestamp,
            keyframes,
            interpolation,
            looping,
            anim_start: Instant::now(),
        }
    }

    #[inline]
    pub fn sample_at_instant(&mut self, current_time: Instant) -> Value {
        if self.looping {
            while (current_time - self.anim_start) >= self.total_duration {
                self.anim_start += self.total_duration
            }
        }
        let current_timestamp = current_time - self.anim_start;

        self.sample_at_timestamp(current_timestamp)
    }

    #[inline]
    pub fn sample_at_timestamp(&self, current_timestamp: Duration) -> Value {
        let (next_index, next_frame) = if current_timestamp >= self.total_duration {
            // If `current_timestamp` is larger than the total duration, then we return the last frame and skip finding it altogether
            let last_index = self.keyframes.len() - 1;
            (last_index, self.keyframes[last_index])
        } else {
            // In order to compute the value of the translation for the current animation time, the following algorithm can be used:
            // - Let the current animation time be given as currentTime.
            // - Compute the next smaller and the next larger element of the times accessor:
            //     previousTime = The largest element from the times accessor that is smaller than the currentTime
            //     nextTime = The smallest element from the times accessor that is larger than the currentTime
            // - Obtain the elements from the translations accessor that correspond to these times:
            //     previousTranslation = The element from the translations accessor that corresponds to the previousTime
            //     nextTranslation = The element from the translations accessor that corresponds to the nextTime
            self.keyframes
                .iter()
                .copied()
                .enumerate()
                .find(|(_, frame)| frame.timestamp > current_timestamp)
                .unwrap()
        };

        // Return early to avoid unnecessary work
        if self.interpolation == Interpolation::Step {
            return next_frame.value;
        }

        let curr_index = ((next_index + self.keyframes.len()) - 1) % self.keyframes.len();
        let curr_frame = self.keyframes[curr_index];

        let delta_time = next_frame.timestamp - curr_frame.timestamp;

        // - Compute the interpolation value. This is a value between 0.0 and 1.0 that describes the relative position of the currentTime, between the previousTime and the nextTime:
        //     interpolationValue = (currentTime - previousTime) / (nextTime - previousTime)
        let t = (current_timestamp - curr_frame.timestamp).div_duration_f32(delta_time);

        match self.interpolation {
            Interpolation::Linear => AnimatedValue::interpolate_linear(curr_frame.value, next_frame.value, t),
            Interpolation::CubicSpline => {
                let delta_time_f32 = delta_time.as_secs_f32();

                let pt = curr_frame.out_tangent.unwrap().mul(delta_time_f32);
                let nt = next_frame.in_tangent.unwrap().mul(delta_time_f32);

                AnimatedValue::interpolate_cubic_spline(curr_frame.value, next_frame.value, pt, nt, t)
            },
            Interpolation::Step => unreachable!(),
        }
    }
}

macro_rules! forward_ref_binop {
    (impl $op: ident, $fun: ident for
        $type: ident$(<$( $gen: ident $(: $first: ident $(+ $trailing: ident )* )? ),+>)?,
        $target: ident$(<$( $target_gen: ident  ),+>)?
) => {
        impl$(<$( $gen $(: $first $(+ $trailing )* )? ),+>)? $op<$target$(<$( $target_gen ),+>)?> for &$type$(<$( $gen ),+>)? {
            type Output = $type$(<$( $gen ),+>)?;

            #[inline]
            fn $fun(self, other: $target$(<$( $target_gen ),+>)?) -> Self::Output {
                $op::$fun(*self, other)
            }
        }

        impl$(<$( $gen $(: $first $(+ $trailing )* )? ),+>)? $op<&$target$(<$( $target_gen ),+>)?> for $type$(<$( $gen ),+>)? {
            type Output = $type$(<$( $gen ),+>)?;

            #[inline]
            fn $fun(self, other: &$target$(<$( $target_gen ),+>)?) -> Self::Output {
                $op::$fun(self, *other)
            }
        }

        impl$(<$( $gen $(: $first $(+ $trailing )* )? ),+>)? $op<&$target$(<$( $target_gen ),+>)?> for &$type$(<$( $gen ),+>)? {
            type Output = $type$(<$( $gen ),+>)?;

            #[inline]
            fn $fun(self, other: &$target$(<$( $target_gen ),+>)?) -> Self::Output {
                $op::$fun(*self, *other)
            }
        }
    };
}

macro_rules! impl_op {
    (
        $target: ident$(<$( $target_gen: ident  ),+>)?,
        (),
        (),
        $type: ident$(<$( $gen: ident $(: $first: ident $(+ $trailing: ident )* )? ),+>)?,
        $new: ident,
        {
            ( $(  $i: ident$([$i_acc: literal])? ),+  )
           $(, ($($j: ident$([$j_acc: literal])? ),+))?
        }
    ) => {};

    (
        $target: ident$(<$( $target_gen: ident  ),+>)?,
        $op: ident,
        $fun: ident,
        $type: ident$(<$( $gen: ident $(: $first: ident $(+ $trailing: ident )* )? ),+>)?,
        $new: ident,
        {
            ($($i: ident$([$i_acc: literal])? ),+),
            ($($j: ident$([$j_acc: literal])? ),+)
        }
    ) => {
        impl$(<$( $gen $(: $first $(+ $trailing )* )? ),+>)? $op<$target$(<$( $target_gen ),+>)?> for $type$(<$( $gen ),+>)? {
            type Output = Self;

            fn $fun(self, rhs: $target$(<$( $target_gen ),+>)?) -> Self::Output {
                Self::$new(
                    $(
                        self.$i$([$i_acc])?.$fun(rhs.$j$([$j_acc])?)
                    ),+
                )
            }
        }

        forward_ref_binop!(impl $op, $fun for $type$(<$( $gen $(: $first $(+ $trailing )* )? ),+>)?, $target$(<$( $target_gen ),+>)?);
    };

    (
        $target: ident$(<$( $target_gen: ident  ),+>)?,
        $op: ident,
        $fun: ident,
        $type: ident$(<$( $gen: ident $(: $first: ident $(+ $trailing: ident )* )? ),+>)?,
        $new: ident,
        {
            ($($i: ident$([$i_acc: literal])? ),+)
        }
    ) => {
        impl$(<$( $gen $(: $first $(+ $trailing )* )? ),+>)? $op<$target> for $type$(<$( $gen ),+>)? {
            type Output = Self;

            fn $fun(self, rhs: $target$(<$( $target_gen ),+>)?) -> Self::Output {
                Self::$new(
                    $(
                        self.$i$([$i_acc])?.$fun(rhs)
                    ),+
                )
            }
        }

        forward_ref_binop!(impl $op, $fun for $type$(<$( $gen $(: $first $(+ $trailing )* )? ),+>)?, $target);
    };

    (
        $target: ident$(<$( $target_gen: ident  ),+>)?,
        ($op: ident $(, $op_trail: ident )*),
        ($fun: ident $(, $fun_trail: ident )*),
        $type: ident$(<$( $gen: ident $(: $first: ident $(+ $trailing: ident )* )? ),+>)?,
        $new: ident,
        {
            ( $(  $i: ident$([$i_acc: literal])? ),+  )
           $(, ($($j: ident$([$j_acc: literal])? ),+))?
        }
    ) => {
        impl_op!(
            $target,
            $op,
            $fun,
            $type$(<$( $gen $(: $first $(+ $trailing )* )? ),+>)?,
            $new,
            {
                ( $(  $i$([$i_acc])? ),+  )
               $(, ($($j$([$j_acc])? ),+))?
            }
        );

        impl_op!(
            $target,
            ($( $op_trail ),*),
            ($( $fun_trail ),*),
            $type$(<$( $gen $(: $first $(+ $trailing )* )? ),+>)?,
            $new,
            {
                ( $(  $i$([$i_acc])? ),+  )
               $(, ($($j$([$j_acc])? ),+))?
            }
        );
    };
}

macro_rules! impl_self_op {
    (
        (),
        (),
        $type: ident$(<$( $gen: ident $(: $first: ident $(+ $trailing: ident )* )? ),+>)?,
        $new: ident,
        ($($i: ident$([$i_acc: literal])? ),+)
    ) => {};

    (
        $op: ident,
        $fun: ident,
        $type: ident$(<$( $gen: ident $(: $first: ident $(+ $trailing: ident )* )? ),+>)?,
        $new: ident,
        ($($i: ident$([$i_acc: literal])? ),+)
    ) => {
        paste::paste! {
            impl_op!(
                $type$(<$( $gen ),+>)?,
                $op,
                $fun,
                $type$(<$( $gen $(: $first $(+ $trailing )* )? ),+>)?,
                $new,
                {
                    ($($i$([$i_acc])? ),+),
                    ($($i$([$i_acc])? ),+)
                }
            );
        }
    };

    (
        ($op: ident $(, $op_trail: ident )*),
        ($fun: ident $(, $fun_trail: ident )*),
        $type: ident$(<$( $gen: ident $(: $first: ident $(+ $trailing: ident )* )? ),+>)?,
        $new: ident,
        ($($i: ident$([$i_acc: literal])? ),+)
    ) => {
        impl_self_op!(
            $op,
            $fun,
            $type$(<$( $gen $(: $first $(+ $trailing )* )? ),+>)?,
            $new,
            ($($i$([$i_acc])? ),+)
        );

        impl_self_op!(
            ($( $op_trail ),*),
            ($( $fun_trail ),*),
            $type$(<$( $gen $(: $first $(+ $trailing )* )? ),+>)?,
            $new,
            ($($i$([$i_acc])? ),+)
        );
    };
}

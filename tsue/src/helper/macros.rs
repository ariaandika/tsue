
macro_rules! derefm {
    ($(<$($g:ident)*>)? |$me:ty| -> $to:ty) => {
        derefm!($(<$($g)*>)? |self:$me| -> $to { self.0 });
    };
    ($(<$($g:ident)*>)? |$id:ident:$me:ty| -> $to:ty { $e:expr }) => {
        impl $(<$($g)*>)? std::ops::Deref for $me {
            type Target = $to;

            fn deref(&$id) -> &Self::Target {
                &$e
            }
        }

        impl $(<$($g)*>)? std::ops::DerefMut for $me {
            fn deref_mut(&mut $id) -> &mut Self::Target {
                &mut $e
            }
        }

        impl $(<$($g)*>)? AsRef<$to> for $me {
            fn as_ref(&$id) -> &T {
                &$e
            }
        }

        impl $(<$($g)*>)? AsMut<$to> for $me {
            fn as_mut(&mut $id) -> &mut T {
                &mut $e
            }
        }
    };
}


pub(crate) use derefm;


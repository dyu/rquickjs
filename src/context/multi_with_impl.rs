use super::{Context, Ctx, MultiWith};
use std::mem;

macro_rules! list {
    ({$($r:ident,)+} => $e:ty) => {
        list!([$($r -> $e,)* ])
    };
    ([$($r:ident -> $e:ty,)* ]) =>{
        ($($e,)*)
    }
}

macro_rules! impl_multi_with{
    ($($t:tt,)*) => {
        impl<'js> MultiWith<'js> for list!({ $($t,)*} => &'js Context) {
            type Arg =  list!({$($t,)*} => Ctx<'js> );

            fn with<R, F: FnOnce(Self::Arg) -> R>(self, f: F) -> R{
                let ($($t,)*) = self;

                $(if self.0.get_runtime_ptr() != $t.get_runtime_ptr(){
                    panic!("Tried to use contexts of different runtimes with eachother");
                })*
                let guard = self.0.rt.lock();
                self.0.reset_stack();
                let res = f(($(Ctx::new($t),)*));
                mem::drop(guard);
                res
            }
        }
    }
}

impl_multi_with!(a, b,);
impl_multi_with!(a, b, c,);
impl_multi_with!(a, b, c, d,);
impl_multi_with!(a, b, c, d, e,);
impl_multi_with!(a, b, c, d, e, f,);
impl_multi_with!(a, b, c, d, e, f, g,);
impl_multi_with!(a, b, c, d, e, f, g, h,);
impl_multi_with!(a, b, c, d, e, f, g, h, i,);
impl_multi_with!(a, b, c, d, e, f, g, h, i, j,);
impl_multi_with!(a, b, c, d, e, f, g, h, i, j, k,);
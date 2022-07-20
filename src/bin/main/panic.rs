pub trait OptionalExt {
    type Value;

    fn unwrap_infallible(self) -> Self::Value;
}

impl<T, E> OptionalExt for Result<T, E> {
    type Value = T;

    #[inline(always)]
    fn unwrap_infallible(self) -> Self::Value {
        match self {
            Ok(x) => x,
            Err(_) => unwrap_failed(),
        }
    }
}

impl<T> OptionalExt for Option<T> {
    type Value = T;

    #[inline(always)]
    fn unwrap_infallible(self) -> Self::Value {
        match self {
            Some(x) => x,
            None => unwrap_failed(),
        }
    }
}

#[inline(never)]
fn unwrap_failed() -> ! {
    extern "Rust" {
        #[link_name = "\n================================\nerror: unwrap was not infallible\n================================"]
        fn undefined() -> !;
    }

    unsafe { undefined() }
}

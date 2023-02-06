/// DO NOT USE FOR ANYTHING OTHER THAN WRITE-FIRST BUFFER OPTIMIZATIONS PLEASE. OTHERWISE I WILL CRY 😢.
macro_rules! uninit {
    () => {
        #[allow(invalid_value)]
        unsafe {
            ::core::mem::MaybeUninit::uninit().assume_init()
        }
    };
    ($t:ty) => {
        #[allow(invalid_value)]
        unsafe {
            ::core::mem::MaybeUninit::<$t>::uninit().assume_init()
        }
    };
}

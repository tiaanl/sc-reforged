#[macro_export]
macro_rules! global {
    ($ty:ty, $scope:ident, $get:ident) => {
        mod global {
            use super::*;

            static mut GLOBAL: *const $ty = std::ptr::null();

            pub struct ScopedGlobal {
                _box: Box<$ty>,
            }
            impl Drop for ScopedGlobal {
                fn drop(&mut self) {
                    unsafe {
                        GLOBAL = std::ptr::null();
                    }
                }
            }

            #[must_use]
            pub fn scoped_global(init: impl FnOnce() -> $ty) -> ScopedGlobal {
                unsafe { debug_assert!(GLOBAL.is_null()) };
                let b = Box::new(init());
                unsafe {
                    GLOBAL = &*b as *const $ty;
                }
                ScopedGlobal { _box: b }
            }

            pub fn get() -> &'static $ty {
                unsafe {
                    debug_assert!(!GLOBAL.is_null(), "Global not initialized");
                    &*GLOBAL
                }
            }
        }

        pub use global::get as $get;
        pub use global::scoped_global as $scope;
    };
}

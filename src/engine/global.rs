#[macro_export]
macro_rules! global {
    ($ty:ty, $scope:ident, $get:ident) => {
        mod global {
            use super::*;

            static mut GLOBAL: *mut $ty = std::ptr::null_mut();

            pub struct ScopedGlobal {
                _box: Box<$ty>,
            }
            impl Drop for ScopedGlobal {
                fn drop(&mut self) {
                    unsafe {
                        GLOBAL = std::ptr::null_mut();
                    }
                }
            }

            #[must_use]
            pub fn scoped_global(init: impl FnOnce() -> $ty) -> ScopedGlobal {
                debug_assert!(unsafe { GLOBAL.is_null() });
                let mut b = Box::new(init());
                unsafe {
                    GLOBAL = &mut *b as *mut $ty;
                }
                ScopedGlobal { _box: b }
            }

            pub fn get() -> &'static mut $ty {
                unsafe {
                    debug_assert!(!GLOBAL.is_null(), "Global not initialized");
                    &mut *GLOBAL
                }
            }
        }

        pub use global::get as $get;
        pub use global::scoped_global as $scope;
    };
}

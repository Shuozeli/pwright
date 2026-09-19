//! Failure-safe publication of an owned Handle into the existing native class.
//!
//! The derived napi-rs conversion relinquishes its Box before fallible instance
//! construction. Here construction and tagging finish before the Box can escape;
//! wrapping is the final fallible step and transfers ownership only on success.
//! NativeHandle exposes borrowed methods, not napi-rs Reference<Class> values,
//! so this wrapper needs neither a native reference nor Reference-map entries.

use super::NativeHandle;
use crate::evaluation_value::status;
use napi::{bindgen_prelude::TypeTag, sys};
use std::{ffi::c_void, marker::PhantomData, ptr};

/// Convert inside the active Node callback scope.
///
/// # Safety
/// `env` must be a live Node environment on its owning JavaScript thread.
pub(crate) unsafe fn to_napi_value(
    env: sys::napi_env,
    handle: NativeHandle,
) -> napi::Result<sys::napi_value> {
    unsafe { to_napi_class(env, "NativeHandle\0", handle) }
}

/// Publish a borrowed-method-only native class with failure-safe Box ownership.
///
/// # Safety
/// The named constructor must describe T in the live, owning Node environment.
pub(crate) unsafe fn to_napi_class<T: TypeTag>(
    env: sys::napi_env,
    class_name: &'static str,
    value: T,
) -> napi::Result<sys::napi_value> {
    publish(
        value,
        &NativeBinding::<T> {
            env,
            class_name,
            kind: PhantomData,
        },
    )
}

trait Binding<T> {
    type Object;

    fn create(&self) -> napi::Result<Self::Object>;
    fn tag(&self, object: &Self::Object) -> napi::Result<()>;

    /// # Safety
    /// `value` is one live Box allocation. Success transfers it to the binding;
    /// failure must leave it untouched and must not install a finalizer for it.
    unsafe fn transfer(&self, object: &Self::Object, value: *mut T) -> napi::Result<()>;
}

fn publish<T, B: Binding<T>>(value: T, binding: &B) -> napi::Result<B::Object> {
    let object = binding.create()?;
    binding.tag(&object)?;
    let value = Box::into_raw(Box::new(value));
    // SAFETY: transfer receives exclusive ownership of this new allocation and
    // its contract distinguishes successful transfer from retained ownership.
    if let Err(error) = unsafe { binding.transfer(&object, value) } {
        // SAFETY: failed transfer retained no pointer or finalizer.
        drop(unsafe { Box::from_raw(value) });
        return Err(error);
    }
    Ok(object)
}

struct NativeBinding<T> {
    env: sys::napi_env,
    class_name: &'static str,
    kind: PhantomData<T>,
}

impl<T: TypeTag> Binding<T> for NativeBinding<T> {
    type Object = sys::napi_value;

    fn create(&self) -> napi::Result<Self::Object> {
        let reference = napi::__private::get_class_constructor(self.class_name)
            .ok_or_else(|| napi::Error::from_reason("Native class constructor is unavailable"))?;
        let mut constructor = ptr::null_mut();
        // SAFETY: the constructor reference belongs to the current environment.
        status(unsafe { sys::napi_get_reference_value(self.env, reference, &mut constructor) })?;
        let mut object = ptr::null_mut();
        let _factory = FactoryScope::enter();
        // SAFETY: the factory flag bypasses the user constructor; object stays
        // callback-scoped and no Rust allocation has been exposed yet.
        status(unsafe {
            sys::napi_new_instance(self.env, constructor, 0, ptr::null(), &mut object)
        })?;
        Ok(object)
    }

    fn tag(&self, object: &Self::Object) -> napi::Result<()> {
        // SAFETY: this is a fresh, untagged native-class instance. Tagging does
        // not require wrapping and grants no authority without a native pointer.
        status(unsafe { sys::napi_type_tag_object(self.env, *object, &T::type_tag()) })
    }

    unsafe fn transfer(&self, object: &Self::Object, value: *mut T) -> napi::Result<()> {
        // SAFETY: Node takes value/finalizer ownership only if napi_wrap succeeds.
        // No napi_ref is requested because no Reference<NativeHandle> is exposed.
        status(unsafe {
            sys::napi_wrap(
                self.env,
                *object,
                value.cast(),
                Some(finalize::<T>),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        })
    }
}

/// Restore even on constructor failure, and preserve a surrounding factory call.
struct FactoryScope(bool);

impl FactoryScope {
    fn enter() -> Self {
        Self(napi::__private::___CALL_FROM_FACTORY.with(|flag| flag.replace(true)))
    }
}

impl Drop for FactoryScope {
    fn drop(&mut self) {
        napi::__private::___CALL_FROM_FACTORY.with(|flag| flag.set(self.0));
    }
}

unsafe extern "C" fn finalize<T>(_env: sys::napi_env, value: *mut c_void, _hint: *mut c_void) {
    // SAFETY: successful transfer gave Node this exact allocation once. Dropping
    // The preparation registry's environment hook clears Node roots before
    // late finalizers. Normal finalization runs on this owning Node thread.
    drop(unsafe { Box::from_raw(value.cast::<T>()) });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc};

    #[derive(Clone, Copy)]
    enum Failure {
        Construct,
        Tag,
        Wrap,
        None,
    }

    struct Owned(Rc<RefCell<usize>>);
    impl Drop for Owned {
        fn drop(&mut self) {
            *self.0.borrow_mut() += 1;
        }
    }

    struct FakeBinding {
        failure: Failure,
        transferred: RefCell<Option<Box<Owned>>>,
    }
    impl Binding<Owned> for FakeBinding {
        type Object = ();
        fn create(&self) -> napi::Result<()> {
            match self.failure {
                Failure::Construct => Err(napi::Error::from_reason("construction failure")),
                Failure::Tag | Failure::Wrap | Failure::None => Ok(()),
            }
        }
        fn tag(&self, _: &()) -> napi::Result<()> {
            match self.failure {
                Failure::Tag => Err(napi::Error::from_reason("tag failure")),
                Failure::Construct | Failure::Wrap | Failure::None => Ok(()),
            }
        }
        unsafe fn transfer(&self, _: &(), value: *mut Owned) -> napi::Result<()> {
            match self.failure {
                Failure::Wrap => Err(napi::Error::from_reason("wrap failure")),
                Failure::Construct | Failure::Tag | Failure::None => {
                    // SAFETY: the successful fake takes exactly the Box supplied
                    // by publish; dropping this field simulates Node finalization.
                    *self.transferred.borrow_mut() = Some(unsafe { Box::from_raw(value) });
                    Ok(())
                }
            }
        }
    }

    #[test]
    fn publication_failures_drop_the_value_without_transferring_its_allocation() {
        for failure in [Failure::Construct, Failure::Tag, Failure::Wrap] {
            // Arrange
            let drops = Rc::new(RefCell::new(0));
            let binding = FakeBinding {
                failure,
                transferred: RefCell::new(None),
            };

            // Act
            let result = publish(Owned(drops.clone()), &binding);

            // Assert
            assert!(result.is_err());
            assert_eq!(*drops.borrow(), 1);
            assert!(binding.transferred.borrow().is_none());
        }
    }

    #[test]
    fn successful_publication_transfers_the_box_until_finalization() {
        // Arrange
        let drops = Rc::new(RefCell::new(0));
        let binding = FakeBinding {
            failure: Failure::None,
            transferred: RefCell::new(None),
        };

        // Act
        let result = publish(Owned(drops.clone()), &binding);

        // Assert
        assert!(result.is_ok());
        assert_eq!(*drops.borrow(), 0);
        let transferred = binding.transferred.borrow_mut().take();
        assert!(transferred.is_some());
        drop(transferred);
        assert_eq!(*drops.borrow(), 1);
    }

    #[test]
    fn factory_scope_restores_the_previous_flag_after_failure() {
        fn fail_inside_factory() -> Result<(), ()> {
            let _scope = FactoryScope::enter();
            assert!(napi::__private::___CALL_FROM_FACTORY.with(|flag| flag.get()));
            Err(())
        }

        for previous in [false, true] {
            // Arrange
            let original =
                napi::__private::___CALL_FROM_FACTORY.with(|flag| flag.replace(previous));

            // Act
            let result = fail_inside_factory();

            // Assert
            assert!(result.is_err());
            assert_eq!(
                napi::__private::___CALL_FROM_FACTORY.with(|flag| flag.get()),
                previous
            );
            napi::__private::___CALL_FROM_FACTORY.with(|flag| flag.set(original));
        }
    }
}

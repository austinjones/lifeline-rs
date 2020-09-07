/// Defines a resource (or channel endpoint) which can be stored on the bus, and how it is taken or cloned.
///
/// Lifeline provides helper macros which can implement the Clone or Take operations:
/// [impl_storage_take!(Struct)](./macro.impl_storage_take.html), and [impl_storage_clone!(Struct)](./macro.impl_storage_clone.html)
///
/// ## Resource Example
/// ```
/// use lifeline::impl_storage_clone;
///
/// #[derive(Debug, Clone)]
/// struct ExampleConfiguration {
///     value: bool
/// }
///
/// impl_storage_clone!(ExampleConfiguration);
/// ```
///
/// ## Channel Example
/// Lifeline also provides [impl_channel_take!(Struct\<T\>)](./macro.impl_channel_take.html) and [impl_channel_clone!(Struct\<T\>)](./macro.impl_channel_take.html),
///  which configure the generic bounds required for a channel implementation:
/// ```
/// use lifeline::impl_channel_take;
/// use std::marker::PhantomData;
///
/// #[derive(Debug)]
/// struct ExampleSender<T> {
///     _t: PhantomData<T>
/// }
///
/// impl_channel_take!(ExampleSender<T>);
/// ```
pub trait Storage: Sized + 'static {
    /// If Self::Tx implements clone, clone it.  Otherwise use Option::take
    fn take_or_clone(res: &mut Option<Self>) -> Option<Self>;

    /// Helper which clones the provided option, requiring that Self: Clone
    fn clone_slot(res: &mut Option<Self>) -> Option<Self>
    where
        Self: Clone,
    {
        res.as_ref().map(|t| t.clone())
    }

    /// Helper which takes the provided option, returning None if it's already been taken
    fn take_slot(res: &mut Option<Self>) -> Option<Self> {
        res.take()
    }
}

/// Specifies that this resource is taken, and is !Clone.
/// ## Example:
/// ```
/// use lifeline::impl_storage_take;
///
/// struct ExampleResource {}
///
/// impl_storage_take!(ExampleResource);
/// ```
#[macro_export]
macro_rules! impl_storage_take {
    ( $name:ty ) => {
        impl $crate::Storage for $name {
            fn take_or_clone(res: &mut Option<Self>) -> Option<Self> {
                Self::take_slot(res)
            }
        }
    };
}

/// Specifies that this channel endpoint (Sender or Receiver) is taken.  Provides a generic type T with the bounds required for the implementation.
/// ## Example:
/// ```
/// use std::marker::PhantomData;
/// use lifeline::impl_channel_take;
///
/// struct ExampleReceiver<T> {
///     _t: PhantomData<T>
/// }
///
/// impl_channel_take!(ExampleReceiver<T>);
/// ```
#[macro_export]
macro_rules! impl_channel_take {
    ( $name:ty ) => {
        impl<T: Send + 'static> $crate::Storage for $name {
            fn take_or_clone(res: &mut Option<Self>) -> Option<Self> {
                Self::take_slot(res)
            }
        }
    };
}

/// Specifies that this resource is cloned.
/// ## Example:
/// ```
/// use lifeline::impl_storage_clone;
///
/// #[derive(Clone)]
/// struct ExampleResource {}
///
/// impl_storage_clone!(ExampleResource);
/// ```
#[macro_export]
macro_rules! impl_storage_clone {
    ( $name:ty ) => {
        impl $crate::Storage for $name {
            fn take_or_clone(res: &mut Option<Self>) -> Option<Self> {
                Self::clone_slot(res)
            }
        }
    };
}

/// Specifies that this channel endpoint (Sender or Receiver) is clonable.  Provides a generic type T with the bounds required for the implementation.
/// ## Example:
/// ```
/// use std::marker::PhantomData;
/// use lifeline::impl_channel_clone;
///
/// struct ExampleReceiver<T> {
///     _t: PhantomData<T>
/// }
///
/// impl<T> Clone for ExampleReceiver<T> {
///     fn clone(&self) -> Self {
///         Self { _t: PhantomData }
///     }   
/// }
///
/// impl_channel_clone!(ExampleReceiver<T>);
/// ```
#[macro_export]
macro_rules! impl_channel_clone {
    ( $name:ty ) => {
        impl<T: Send + 'static> $crate::Storage for $name {
            fn take_or_clone(res: &mut Option<Self>) -> Option<Self> {
                Self::clone_slot(res)
            }
        }
    };
}

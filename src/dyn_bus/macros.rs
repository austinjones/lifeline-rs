/// Defines a lifeline bus: it's struct, `Bus` impl, and `DynBus` impl.
///
/// ## Examples
/// ```
/// use lifeline::prelude::*;
/// use tokio::sync::mpsc;
///
/// lifeline_bus!(pub struct ExampleBus);
///
/// // carry ExampleMessage on the bus, using a tokio mpsc sender.
/// #[derive(Debug)]
/// pub struct ExampleMessage {}
/// impl Message<ExampleBus> for ExampleMessage {
///     type Channel = mpsc::Sender<Self>;
/// }
/// ```
/// You can also define private structs:
/// ```
/// use lifeline::prelude::*;
///
/// lifeline_bus!(struct PrivateExampleBus);
/// ```
/// You can also define generics (which are constrained to Debug):
/// ```
/// use lifeline::prelude::*;
/// lifeline_bus!(pub struct ExampleBus<T>);
/// ```
/// ## Prelude, auto-imports, and rust-analyzer
/// Unfortunately, rust-analyzer doesn't handle auto-imports for structures defined in macros.
/// There is an ergonomic solution: define a prelude module in your crate root, and `pub use` all your bus structs.
/// If you want, you can `pub use lifeline::prelude::*` as well.
#[macro_export]
macro_rules! lifeline_bus (
    (struct $name:ident $(< $( $gen:ident ),+ >)? ) => {
        lifeline_bus! { () struct $name $(< $( $gen ),+ >)? }
    };

    (pub struct $name:ident $(< $( $gen:ident ),+ >)* ) => {
        lifeline_bus! { (pub) struct $name $(< $( $gen ),+ >)* }
    };

    (($($vis:tt)*) struct $name:ident $(< $( $gen:ident ),+ >)? ) => {
        #[derive(Debug)]
        #[allow(non_snake_case)]
        $($vis)* struct $name $(< $( $gen: std::fmt::Debug ),+ >)? {
            storage: $crate::dyn_bus::DynBusStorage<Self>,
            $(
                $( $gen: std::marker::PhantomData<$gen> ),+
            )?
        }

        impl$(< $( $gen: std::fmt::Debug ),+ >)? std::default::Default for $name $(< $( $gen ),+ >)? {
            fn default() -> Self {
                Self {
                    storage: $crate::dyn_bus::DynBusStorage::default(),
                    $(
                        $( $gen: std::marker::PhantomData::<$gen> ),+
                    )?
                }
            }
        }

        impl$(< $( $gen: std::fmt::Debug ),+ >)? $crate::dyn_bus::DynBus for $name$(< $( $gen ),+ >)? {
            fn store_rx<Msg>(&self, rx: <Msg::Channel as $crate::Channel>::Rx) -> Result<(), $crate::error::AlreadyLinkedError>
                where Msg: $crate::Message<Self> + 'static
            {
                self.storage().store_channel::<Msg, Msg::Channel, Self>(Some(rx), None)
            }

            fn store_tx<Msg>(&self, tx: <Msg::Channel as $crate::Channel>::Tx) -> Result<(), $crate::error::AlreadyLinkedError>
                where Msg: $crate::Message<Self> + 'static
            {
                self.storage().store_channel::<Msg, Msg::Channel, Self>(None, Some(tx))
            }

            fn store_channel<Msg>(
                &self,
                rx: <Msg::Channel as $crate::Channel>::Rx,
                tx: <Msg::Channel as $crate::Channel>::Tx
            ) -> Result<(), $crate::error::AlreadyLinkedError>
                where Msg: $crate::Message<Self> + 'static
            {
                self.storage().store_channel::<Msg, Msg::Channel, Self>(Some(rx), Some(tx))
            }

            fn store_resource<R: $crate::Resource<Self>>(&self, resource: R) {
                self.storage.store_resource::<R, Self>(resource)
            }

            fn storage(&self) -> &$crate::dyn_bus::DynBusStorage<Self> {
                &self.storage
            }
        }
    }
);

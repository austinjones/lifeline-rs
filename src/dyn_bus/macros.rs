#[macro_export]
macro_rules! lifeline_bus (
            // match one or more generics separated by a comma
    ($name:ident $(< $( $gen:ident ),+ >)? ) => {
        lifeline_bus! { () struct $name $(< $( $gen ),+ >)? }
    };

    (pub $name:ident $(< $( $gen:ident ),+ >)* ) => {
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

            // fn take_channel<Msg, Source>(
            //     &self,
            //     other: &Source,
            // ) -> Result<(), $crate::TakeChannelError>
            // where
            //     Msg: $crate::Message<Self> + 'static,
            //     Msg: $crate::Message<Source, Channel = <Msg as $crate::Message<Self>>::Channel>,
            //     Source: $crate::dyn_bus::DynBus
            // {
            //     self.storage.take_channel::<Msg, Source, Self, <Msg as $crate::Message<Self>>::Channel>(other, true, true)
            // }

            // fn take_rx<Msg, Source>(
            //     &self,
            //     other: &Source,
            // ) -> Result<(), $crate::TakeChannelError>
            // where
            //     Msg: $crate::Message<Self> + 'static,
            //     Msg: $crate::Message<Source, Channel = <Msg as $crate::Message<Self>>::Channel>,
            //     Source: $crate::dyn_bus::DynBus
            // {
            //     self.storage.take_channel::<Msg, Source, Self, <Msg as $crate::Message<Self>>::Channel>(other, true, false)
            // }

            // fn take_tx<Msg, Source>(
            //     &self,
            //     other: &Source,
            // ) -> Result<(), $crate::TakeChannelError>
            // where
            //     Msg: $crate::Message<Self> + 'static,
            //     Msg: $crate::Message<Source, Channel = <Msg as $crate::Message<Self>>::Channel>,
            //     Source: $crate::dyn_bus::DynBus
            // {
            //     self.storage.take_channel::<Msg, Source, Self, <Msg as $crate::Message<Self>>::Channel>(other, false, true)
            // }

            // fn take_resource<Res, Source>(
            //     &self,
            //     other: &Source,
            // ) -> Result<(), $crate::error::TakeResourceError>
            // where
            //     Res: $crate::Storage,
            //     Res: $crate::Resource<Source>,
            //     Res: $crate::Resource<Self>,
            //     Source: $crate::dyn_bus::DynBus
            // {
            //     self.storage.take_resource::<Res, Source, Self>(other)
            // }

            fn storage(&self) -> &$crate::dyn_bus::DynBusStorage<Self> {
                &self.storage
            }
        }
    }
    // ($name:ident) => {
    //     #[derive(Debug, Default)]
    //     struct $name {
    //         storage: $crate::dyn_bus::DynBusStorage<Self>,
    //     }

    //     impl $crate::dyn_bus::DynBus for $name {
    //         fn storage(&self) -> &$crate::dyn_bus::DynBusStorage<Self> {
    //             &self.storage
    //         }
    //     }
    // };
);

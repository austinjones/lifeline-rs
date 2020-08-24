use crate::{
    spawn::{spawn_task, task_name},
    Bus, Lifeline,
};
use futures::Future;
use log::{debug, error};
use std::{any::TypeId, fmt::Debug};

pub trait Service: Task {
    type Bus: Bus;
    type Lifeline;

    fn spawn(bus: &Self::Bus) -> Self::Lifeline;
}

pub trait DefaultService: Service {
    fn spawn_default() -> (Self::Bus, Self::Lifeline);
}

impl<T> DefaultService for T
where
    T: Service,
{
    fn spawn_default() -> (Self::Bus, Self::Lifeline) {
        let bus = Self::Bus::default();
        let lifeline = Self::spawn(&bus);

        (bus, lifeline)
    }
}

pub trait FromCarrier<IntoBus: Bus>: Bus + Task + Sized {
    type Lifeline;

    fn carry_from(&self, from: &IntoBus) -> Self::Lifeline;
}

pub trait IntoCarrier<IntoBus: Bus>: Bus + Task + Sized {
    type Lifeline;

    fn carry_into(&self, into: &IntoBus) -> Self::Lifeline;
}

impl<F, I> IntoCarrier<I> for F
where
    I: FromCarrier<F>,
    F: Bus,
    I: Bus,
{
    type Lifeline = <I as FromCarrier<F>>::Lifeline;

    fn carry_into(&self, into: &I) -> Self::Lifeline {
        into.carry_from(self)
    }
}

pub trait DefaultCarrier<FromBus: Bus>: FromCarrier<FromBus> {
    fn carry_default() -> (Self, FromBus, Self::Lifeline) {
        let into = Self::default();
        let from = FromBus::default();
        let lifeline = into.carry_from(&from);

        (into, from, lifeline)
    }
}

pub trait Task {
    fn task<Out>(name: &str, fut: impl Future<Output = Out> + Send + 'static) -> Lifeline
    where
        Out: Debug + Send + 'static,
        Self: Sized,
    {
        let service_name = task_name::<Self>(name);
        spawn_task(service_name, fut)
    }

    fn try_task<Out>(
        name: &str,
        fut: impl Future<Output = anyhow::Result<Out>> + Send + 'static,
    ) -> Lifeline
    where
        Out: Debug + 'static,
        Self: Sized,
    {
        let service_name = task_name::<Self>(name);
        spawn_task(service_name.clone(), async move {
            match fut.await {
                Ok(val) => {
                    if TypeId::of::<Out>() != TypeId::of::<()>() {
                        debug!("OK {}: {:?}", service_name, val);
                    } else {
                        debug!("OK {}", service_name);
                    }
                }
                Err(err) => {
                    error!("ERR: {}: {}", service_name, err);
                }
            }
        })
    }
}

impl<T> Task for T {}

// #[async_trait]
// pub trait AsyncService: Task {
//     type Bus: Bus + Send + Sync + 'static;
//     type Lifeline;

//     async fn spawn(bus: &Self::Bus) -> Self::Lifeline;

//     async fn spawn_bus() -> (Self::Bus, Self::Lifeline) {
//         let bus = Self::Bus::default();
//         let lifeline = Self::spawn(&bus).await;

//         (bus, lifeline)
//     }
// }

// #[async_trait]
// pub trait AsyncCarrier: Task {
//     type From: Bus + Send + Sync + 'static;
//     type Into: Bus + Send + Sync + 'static;
//     type Lifeline;

//     async fn carry(from_bus: &Self::From, to_bus: &Self::Into) -> Self::Lifeline;

//     async fn carry_bus() -> (Self::From, Self::Into, Self::Lifeline) {
//         let from = Self::From::default();
//         let into = Self::Into::default();
//         let lifeline = Self::carry(&from, &into).await;

//         (from, into, lifeline)
//     }
// }

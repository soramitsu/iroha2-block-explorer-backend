use super::logger;
use actix::prelude::{Actor, Addr, AsyncContext, Context, Handler, Message};
use color_eyre::eyre::eyre;
use core::time::Duration;
use iroha_client::client::Client as IrohaClient;
use rand::{rngs::ThreadRng, Rng};
use std::sync::{Arc, Mutex, MutexGuard};

const DEV_ACTOR_WORK_INTERVAL_MS: u64 = 1500;

pub struct DevActor {
    client: Arc<Mutex<IrohaClient>>,
    rng: ThreadRng,
}

impl DevActor {
    pub fn start(client: Arc<Mutex<IrohaClient>>) -> Addr<Self> {
        Actor::start(Self {
            client,
            rng: rand::thread_rng(),
        })
    }

    fn do_random_stuff(&mut self) -> color_eyre::Result<()> {
        let some_action = RandomAction::new();
        logger::info!("Doing: {:?}", some_action);
        {
            let mut client = self
                .client
                .lock()
                .map_err(|err| eyre!("Failed to lock client mutex: {:?}", err))?;
            some_action.apply(&mut client, &mut self.rng)
        }
    }
}

impl Actor for DevActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        logger::info!("DevActor is started");
        ctx.notify(DoSomething);
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct DoSomething;

impl Handler<DoSomething> for DevActor {
    type Result = ();

    fn handle(&mut self, _msg: DoSomething, ctx: &mut Self::Context) -> Self::Result {
        match self.do_random_stuff() {
            Ok(()) => (),
            Err(err) => logger::error!("Failed to do random stuff: {}", err),
        };

        ctx.notify_later(
            DoSomething,
            Duration::from_millis(DEV_ACTOR_WORK_INTERVAL_MS),
        );
    }
}

#[derive(Clone, Copy, Debug)]
enum RandomAction {
    RegisterDomain,
}

impl RandomAction {
    fn new() -> Self {
        Self::RegisterDomain
    }

    fn apply(
        self,
        client: &mut MutexGuard<IrohaClient>,
        rng: &mut ThreadRng,
    ) -> color_eyre::Result<()> {
        use faker_rand::fr_fr::names::FirstName;
        use iroha_data_model::prelude::{DomainId, RegisterBox};

        match self {
            Self::RegisterDomain => {
                let domain_name: FirstName = rng.gen();
                let new_domain_id: DomainId = domain_name.to_string().to_lowercase().parse()?;
                let create_domain = RegisterBox::new(new_domain_id);

                client.submit(create_domain)?;

                // do
                Ok(())
            }
        }
    }
}

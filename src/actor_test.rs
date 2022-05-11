use std::time::Duration;

use actix::{
    fut::wrap_future, prelude::*, ActorFutureExt, ContextFutureSpawner, ResponseActFuture,
};

struct Foo {
    counter: u32,
}

impl Foo {
    // async fn do_work(&self) -> () {
    //     // self.counter += 1;
    // }
}

impl Actor for Foo {
    type Context = Context<Self>;
}

#[derive(Message)]
#[rtype(result = "()")]
struct DoWork;

impl DoWork {
    async fn do_work(&mut self) -> () {}
}

impl Handler<DoWork> for Foo {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, mut msg: DoWork, _ctx: &mut Self::Context) -> Self::Result {
        let fut = async move {
            msg.do_work().await;
            msg
        }
        .into_actor(self)
        .map(|res, _act, ctx| {
            ctx.notify_later(res, Duration::from_millis(2000));
        });
        // let fut = wrap_future::<_, Self>(fut);
        // let fut = fut.map(|res, _act, _ctx| {
        //     ctx.notify_later(res, Duration::from_millis(2000));
        // });

        Box::pin(fut)
        // ctx.spawn(fut);
    }
}

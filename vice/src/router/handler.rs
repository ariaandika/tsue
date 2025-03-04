//! functional route
use crate::{
    http::{FromRequest, FromRequestParts, IntoResponse, Request, Response},
    util::futures::{FutureExt, MapInfallible},
};
use hyper::service::Service;
use std::{convert::Infallible, marker::PhantomData};

pub use arg_future::{Fd, FdMap, Fr, FrMap, Frp, FrpMap};

pub fn get<F,S>(f: F) -> FnService<F, S> {
    FnService { inner: f, _s: PhantomData }
}

#[derive(Clone)]
pub struct FnService<F,S> {
    inner: F,
    _s: PhantomData<S>
}

impl<F,S> Service<Request> for FnService<F,S>
where
    F: FnHandler<S>,
{
    type Response = Response;
    type Error = Infallible;
    type Future = MapInfallible<<F as FnHandler<S>>::Future>;

    fn call(&self, req: Request) -> Self::Future {
        self.inner.call(req).map_infallible()
    }
}

pub trait FnHandler<S> {
    type Future: Future<Output = Response>;
    fn call(&self, req: Request) -> Self::Future;
}

impl<F,A,R> FnHandler<(A,)> for F
where
    F: Clone + Send + 'static + Fn(A),
    F::Output: Future<Output = R>,
    R: IntoResponse,
    A: FromRequest,
{
    type Future = Fd<FrMap<A>, A, F, F::Output>;
    fn call(&self, req: Request) -> Self::Future {
        fn map<A,F,Fut>(a: A, me: F) -> Fut where F: FnOnce(A) -> Fut { me(a) }
        let (parts,body) = req.into_parts();
        Fd::new(FrMap::new(parts, body), self.clone(), map)
    }
}

macro_rules! fn_handler {
    {
        [$($a:ident,)*]
        [$($aa:ident,)*]
        [$($t:tt)*]
        [$($tt:tt)*]
        type Future = $type:ty;
        ($self:ident,$parts:ident,$b:ident,$map:ident) => $body:expr
    } => {
        impl<F,$($aa,)*A,Fut,R> FnHandler<($($aa,)*A,)> for F
        where
            F: Clone + Send + 'static + Fn($($aa,)*A) -> Fut,
            Fut: Future<Output = R>,
            R: IntoResponse,
            $($aa: FromRequestParts,)*
            A: FromRequest,
        {
            type Future = $type;
            fn call(&$self, req: Request) -> Self::Future {
                fn $map<$($aa,)*A,F,Fut>(($($t)*,a): ($($tt)*,A), me: F) -> Fut
                where
                    F: FnOnce($($aa,)*A) -> Fut,
                {
                    me($($a,)*a)
                }
                let ($parts,$b) = req.into_parts();
                $body
            }
        }
    };
}

fn_handler! {
    [a1,] [A1,] [a1] [A1]
    type Future = Fd<
        Fr<A1, FrpMap<A1>, A>,
        (A1, A), F, Fut
    >;
    (self,parts,body,map) => Fd::new(Fr::new(FrpMap::new(parts), body), self.clone(), map)
}

fn_handler! {
    [a1,a2,] [A1,A2,] [(a1,a2)] [(A1,A2)]
    type Future = Fd<
        Fr<(A1, A2), Frp<A1, FrpMap<A1>, A2>, A>,
        ((A1, A2), A), F, Fut
    >;
    (self,parts,body,map) => Fd::new(Fr::new(Frp::new(FrpMap::new(parts)), body), self.clone(), map)
}

fn_handler! {
    [a1,a2,a3,] [A1,A2,A3,] [((a1,a2),a3)] [((A1,A2),A3)]
    type Future = Fd<
        Fr<
            ((A1, A2), A3),
            Frp<
                (A1, A2),
                Frp<A1, FrpMap<A1>, A2>,
                A3
            >,
            A
        >,
        (((A1, A2), A3), A), F, Fut
    >;
    (self,parts,body,map) => Fd::new(Fr::new(Frp::new(Frp::new(FrpMap::new(parts))), body), self.clone(), map)
}

fn_handler! {
    [a1,a2,a3,a4,] [A1,A2,A3,A4,] [(((a1,a2),a3),a4)] [(((A1,A2),A3),A4)]
    type Future = Fd<
        Fr<
            (((A1, A2), A3), A4),
            Frp<((A1, A2), A3), Frp<(A1, A2), Frp<A1, FrpMap<A1>, A2>, A3>, A4>,
            A,
        >,
        ((((A1, A2), A3), A4), A),
        F,
        Fut,
    >;
    (self,parts,body,map) => Fd::new(Fr::new(Frp::new(Frp::new(Frp::new(FrpMap::new(parts)))), body), self.clone(), map)
}

fn_handler! {
    [a1,a2,a3,a4,a5,] [A1,A2,A3,A4,A5,] [((((a1,a2),a3),a4),a5)] [((((A1,A2),A3),A4),A5)]
    type Future = Fd<
        Fr<
            ((((A1, A2), A3), A4), A5),
            Frp<
                (((A1, A2), A3), A4),
                Frp<((A1, A2), A3), Frp<(A1, A2), Frp<A1, FrpMap<A1>, A2>, A3>, A4>,
                A5,
            >,
            A,
        >,
        (((((A1, A2), A3), A4), A5), A),
        F,
        Fut,
    >;
    (self,parts,body,map) => Fd::new(Fr::new(Frp::new(Frp::new(Frp::new(Frp::new(FrpMap::new(parts))))), body), self.clone(), map)
}

mod arg_future {
    use crate::http::{ReqBody, FromRequest, FromRequestParts, IntoResponse, Response};
    use http::request;

    // ---

    pin_project_lite::pin_project! {
        /// call handler with captured Parts
        pub struct FdMap<Fd,Args,Fut> {
            parts: request::Parts,
            args: Option<Args>,
            inner: Option<Fd>,
            #[pin]
            state: FdMapState<Fut>,
        }

    }

    pin_project_lite::pin_project! {
        #[project = FdMapStateProj]
        enum FdMapState<Fut> {
            Init,
            Fut {
                #[pin] f: Fut,
            },
        }
    }

    impl<Fd, Args, Fut> FdMap<Fd, Args, Fut> {
        pub fn new(parts: request::Parts, args: Args, inner: Fd) -> FdMap<Fd, Args, Fut> {
            FdMap { parts, args: Some(args), inner: Some(inner), state: FdMapState::Init }
        }
    }

    impl<Fd,Args,Fut> Future for FdMap<Fd,Args,Fut>
    where
        Fd: FnOnce(Args) -> Fut,
        Fut: Future,
        Fut::Output: IntoResponse,
    {
        type Output = Response;

        fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
            use std::task::Poll::*;
            let mut me = self.as_mut().project();
            loop {
                match me.state.as_mut().project() {
                    FdMapStateProj::Init => me.state.set(FdMapState::Fut {
                        f: (me.inner.take().expect("poll after complete"))
                            (me.args.take().expect("poll after complete")),
                    }),
                    FdMapStateProj::Fut { f } => match f.poll(cx) {
                        Ready(ok) => {
                            let res = ok.into_response(me.parts);
                            return Ready(res)
                        }
                        Pending => return Pending,
                    },
                }
            }
        }
    }

    // ---

    pin_project_lite::pin_project! {
        #[project = FdProj]
        #[project_replace = FdRepl]
        pub enum Fd<Fargs,Args,Fd1,Fut>
        where
            Fargs: Future<Output = (request::Parts,Result<Args, Response>)>,
        {
            First { #[pin] f: Fargs, inner: Fd1, mapper: fn(Args,Fd1) -> Fut },
            Second { #[pin] f: Fut, parts: request::Parts },
            Invalid,
        }
    }

    impl<Fargs,Args,Fd1,Fut> Fd<Fargs,Args,Fd1,Fut>
    where
        Fargs: Future<Output = (request::Parts,Result<Args, Response>)>,
        Fut: Future,
        Fut::Output: IntoResponse,
    {
        pub fn new(f: Fargs, inner: Fd1, mapper: fn(Args,Fd1) -> Fut) -> Fd<Fargs, Args, Fd1, Fut> {
            Fd::First { f, inner, mapper }
        }
    }

    impl<Fargs,Args,Fd1,Fut> Future for Fd<Fargs,Args,Fd1,Fut>
    where
        Fargs: Future<Output = (request::Parts,Result<Args, Response>)>,
        Fut: Future,
        Fut::Output: IntoResponse,
    {
        type Output = Response;

        fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
            use std::task::Poll::*;
            loop {
                match self.as_mut().project() {
                    FdProj::First { f, .. } => {
                        match f.poll(cx) {
                            Ready((parts,args)) => {
                                let args = match args {
                                    Ok(args) => args,
                                    Err(err) => return Ready(err),
                                };
                                let FdRepl::First { inner, mapper, .. } = self.as_mut().project_replace(Fd::Invalid)
                                else { unreachable!() };
                                self.set(Fd::Second { f: mapper(args,inner), parts });
                            },
                            Pending => return Pending,
                        }
                    }
                    FdProj::Second { f, mut parts } => {
                        return match f.poll(cx) {
                            Ready(res) => Ready(res.into_response(&mut parts)),
                            Pending => Pending,
                        }
                    }
                    _ => todo!()
                }
            }
        }
    }

    // ---

    pin_project_lite::pin_project! {
        /// call FromRequest with captured Parts
        pub struct FrMap<Fr>
        where
            Fr: FromRequest,
        {
            parts: Option<request::Parts>,
            body: Option<ReqBody>,
            #[pin]
            state: FrMapState<Fr>,
        }
    }

    // NOTE: Init state is required to use pinned `Parts`

    pin_project_lite::pin_project! {
        #[project = FrMapStateProj]
        enum FrMapState<Fr>
        where
            Fr: FromRequest,
        {
            Init,
            Fut {
                #[pin] f: Fr::Future,
            },
        }
    }

    impl<Fr> FrMap<Fr>
    where
        Fr: FromRequest,
    {
        pub fn new(parts: request::Parts, body: ReqBody) -> FrMap<Fr> {
            FrMap { parts: Some(parts), state: FrMapState::Init, body: Some(body) }
        }
    }

    impl<Fr> Future for FrMap<Fr>
    where
        Fr: FromRequest,
    {
        type Output = (request::Parts,Result<Fr,Response>);

        fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
            use std::task::Poll::*;
            let mut me = self.as_mut().project();
            loop {
                match me.state.as_mut().project() {
                    FrMapStateProj::Init => me.state.set(FrMapState::Fut {
                        f: Fr::from_request(
                           &mut me.parts.as_mut().expect("poll after complete"),
                           me.body.take().expect("poll after complete")
                        ),
                    }),
                    FrMapStateProj::Fut { f } => match f.poll(cx) {
                        Ready(ok) => {
                            let mut parts = me.parts.take().expect("poll after complete");
                            let result = match ok {
                                Ok(ok) => Ok(ok),
                                Err(err) => Err(err.into_response(&mut parts)),
                            };
                            return Ready((parts,result))
                        }
                        Pending => return Pending,
                    },
                }
            }
        }
    }

    // ---

    pin_project_lite::pin_project! {
        #[project = FrProj]
        #[project_replace = FrRepl]
        pub enum Fr<Frp1,FutMap,Fr1>
        where
            Fr1: FromRequest,
            FutMap: Future<Output = (request::Parts,Result<Frp1,Response>)>
        {
            First { #[pin] f: FutMap, body: Option<ReqBody>, },
            Second { #[pin] f: FrMap<Fr1>, frp1: Frp1, },
            Invalid,
        }
    }

    impl<Frp1, FutMap, Fr1> Fr<Frp1, FutMap, Fr1>
    where
        Fr1: FromRequest,
        FutMap: Future<Output = (request::Parts, Result<Frp1, Response>)>,
    {
        pub fn new(f: FutMap, body: ReqBody) -> Fr<Frp1, FutMap, Fr1> {
            Fr::First { f, body: Some(body) }
        }
    }

    impl<Frp1,FutMap,Fr1> Future for Fr<Frp1,FutMap,Fr1>
    where
        Fr1: FromRequest,
        FutMap: Future<Output = (request::Parts,Result<Frp1,Response>)>,
    {
        type Output = (request::Parts,Result<(Frp1,Fr1),Response>);

        fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
            use std::task::Poll::*;
            loop {
                match self.as_mut().project() {
                    FrProj::First { f, body } => {
                        match f.poll(cx) {
                            Ready((parts,frp1result)) => {
                                let frp1 = match frp1result {
                                    Ok(frp1) => frp1,
                                    Err(err) => return Ready((parts,Err(err))),
                                };
                                let body = body.take().expect("poll after complete");
                                self.set(Fr::Second { f: FrMap::new(parts, body), frp1 });
                            }
                            Pending => return Pending,
                        }
                    }
                    FrProj::Second { f, .. } => {
                        match f.poll(cx) {
                            Ready((parts,fr1result)) => {
                                let fr1 = match fr1result {
                                    Ok(fr1) => fr1,
                                    Err(err) => return Ready((parts,Err(err))),
                                };
                                let FrRepl::Second { frp1, .. } = self.as_mut().project_replace(Fr::Invalid) else {
                                    unreachable!()
                                };
                                return Ready((parts,Ok((frp1,fr1))))
                            }
                            Pending => return Pending,
                        }
                    }
                    FrProj::Invalid => panic!("poll after complete")
                }
            }
        }
    }

    // ---

    pin_project_lite::pin_project! {
        /// call FromRequestParts with captured Parts
        pub struct FrpMap<Frp1>
        where
            Frp1: FromRequestParts,
        {
            parts: Option<request::Parts>,
            #[pin]
            state: FrpMapState<Frp1>,
        }
    }

    pin_project_lite::pin_project! {
        #[project = FrpMapStateProj]
        enum FrpMapState<Frp1>
        where
            Frp1: FromRequestParts,
        {
            Init,
            Fut {
                #[pin] f: Frp1::Future,
            },
        }
    }

    impl<Frp1> FrpMap<Frp1>
    where
        Frp1: FromRequestParts,
    {
        pub fn new(parts: request::Parts) -> FrpMap<Frp1> {
            FrpMap { parts: Some(parts), state: FrpMapState::Init }
        }
    }

    impl<Frp1> Future for FrpMap<Frp1>
    where
        Frp1: FromRequestParts,
    {
        type Output = (request::Parts,Result<Frp1,Response>);

        fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
            use std::task::Poll::*;
            let mut me = self.as_mut().project();
            loop {
                match me.state.as_mut().project() {
                    FrpMapStateProj::Init => me.state.set(FrpMapState::Fut {
                        f: Frp1::from_request_parts(&mut me.parts.as_mut().expect("poll after complete")),
                    }),
                    FrpMapStateProj::Fut { f } => match f.poll(cx) {
                        Ready(ok) => {
                            let mut parts = me.parts.take().expect("poll after complete");
                            let result = match ok {
                                Ok(ok) => Ok(ok),
                                Err(err) => Err(err.into_response(&mut parts)),
                            };
                            return Ready((parts,result))
                        }
                        Pending => return Pending,
                    },
                }
            }
        }
    }

    // ---

    pin_project_lite::pin_project! {
        #[project = FrpProj]
        #[project_replace = FrpRepl]
        pub enum Frp<Frp1,FutMap,Frp2>
        where
            Frp2: FromRequestParts,
            FutMap: Future<Output = (request::Parts,Result<Frp1,Response>)>
        {
            First { #[pin] f: FutMap, },
            Second { #[pin] f: FrpMap<Frp2>, frp1: Frp1, },
            Invalid,
        }
    }

    impl<Frp1,FutMap,Frp2> Frp<Frp1,FutMap,Frp2>
    where
        Frp2: FromRequestParts,
        FutMap: Future<Output = (request::Parts,Result<Frp1,Response>)>
    {
        pub fn new(f: FutMap) -> Frp<Frp1, FutMap, Frp2> {
            Frp::First { f }
        }
    }

    impl<Frp1,FutMap,Frp2> Future for Frp<Frp1,FutMap,Frp2>
    where
        Frp2: FromRequestParts,
        FutMap: Future<Output = (request::Parts,Result<Frp1,Response>)>
    {
        type Output = (request::Parts,Result<(Frp1,Frp2),Response>);

        fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
            use std::task::Poll::*;
            loop {
                match self.as_mut().project() {
                    FrpProj::First { f } => {
                        match f.poll(cx) {
                            Ready((parts,frp1result)) => {
                                let frp1 = match frp1result {
                                    Ok(fr1) => fr1,
                                    Err(err) => return Ready((parts,Err(err))),
                                };
                                self.set(Frp::Second { f: FrpMap::new(parts), frp1 });
                            }
                            Pending => return Pending,
                        }
                    }
                    FrpProj::Second { f, .. } => {
                        match f.poll(cx) {
                            Ready((parts,frp2result)) => {
                                let frp2 = match frp2result {
                                    Ok(frp2) => frp2,
                                    Err(err) => return Ready((parts,Err(err))),
                                };
                                let FrpRepl::Second { frp1, .. } = self.as_mut().project_replace(Frp::Invalid) else {
                                    unreachable!()
                                };
                                return Ready((parts,Ok((frp1,frp2))))
                            }
                            Pending => return Pending,
                        }
                    }
                    FrpProj::Invalid => panic!("poll after complete")
                }
            }
        }
    }
}


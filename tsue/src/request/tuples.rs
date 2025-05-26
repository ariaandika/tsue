use http::request;
use std::{
    pin::Pin,
    task::{
        Context,
        Poll::{self, *},
        ready,
    },
};

use super::{Body, FromRequest, FromRequestParts, Request};
use crate::response::{IntoResponse, Response};

// ===== Macros =====

macro_rules! ty2 {
    (
        $me:ident, $dep:ident, $($r:ident,)*
        |$p:pat_param,$b:pat_param|$body:expr
    ) => {
        #[allow(non_snake_case)]
        mod $me {
            pub type F<$dep, $($r,)* $me> = super::Frp<super::$dep::F<$dep, $($r,)*>, super::$dep::T<$dep, $($r,)*>, $me>;
            pub type T<$dep, $($r,)* $me> = (super::$dep::T<$dep, $($r,)*>, $me);
            pub type M<$dep, $($r,)* $me, A> = super::Fr<F<$dep, $($r,)* $me>, T<$dep, $($r,)* $me>, A, fn(T<$dep, $($r,)* $me>, A) -> ($dep, $($r,)* $me, A)>;
        }

        impl<$dep, $($r,)* $me, A> FromRequest for ($dep, $($r,)* $me, A)
        where
            $dep: FromRequestParts,
            $dep::Error: IntoResponse,
            $(
                $r: FromRequestParts,
                $r::Error: IntoResponse,
            )*
            $me: FromRequestParts,
            $me::Error: IntoResponse,
            A: FromRequest,
            A::Error: IntoResponse,
        {
            type Error = Response;

            type Future = self::$me::M<$dep, $($r,)* $me, A>;

            fn from_request(req: Request) -> Self::Future {
                let ($p, $b) = req.into_parts();$body
            }
        }
    };
}

// ===== Type Aliases =====

#[allow(non_snake_case)]
mod M1 {
    pub type F<A1> = super::FrpCall<A1>;
    pub type T<A1> = A1;
}

ty2!{
    M2,M1,
    |parts,body|fr(fp(fc(parts)), body, |(a1, a2), a| (a1, a2, a))
}
ty2!{
    M3,M2,A1,
    |parts,body|{
        fr(fp(fp(fc(parts))), body, |((a1, a2), a3), a| (a1, a2, a3, a))
    }
}
ty2!{
    M4,M3,A1,A2,
    |parts,body|{
        fr(fp(fp(fp(fc(parts)))), body, |(((a1, a2), a3), a4), a| (a1, a2, a3, a4, a))
    }
}
ty2!{
    M5,M4,A1,A2,A3,
    |parts,body|{
        fr(fp(fp(fp(fp(fc(parts))))), body, |((((a1, a2), a3), a4), a5), a| (a1, a2, a3, a4, a5, a))
    }
}
ty2!{
    M6,M5,A1,A2,A3,A4,
    |parts,body|{
        fr(fp(fp(fp(fp(fp(fc(parts)))))), body, |(((((a1, a2), a3), a4), a5), a6), a| (a1, a2, a3, a4, a5, a6, a))
    }
}
ty2!{
    M7,M6,A1,A2,A3,A4,A5,
    |parts,body|{
        fr(
            fp(fp(fp(fp(fp(fp(fc(parts))))))),
            body,
            |((((((a1, a2), a3), a4), a5), a6), a7), a| (a1, a2, a3, a4, a5, a6, a7, a)
        )
    }
}
ty2!{
    M8,M7,A1,A2,A3,A4,A5,A6,
    |parts,body|{
        fr(
            fp(fp(fp(fp(fp(fp(fp(fc(parts)))))))),
            body,
            |(((((((a1, a2), a3), a4), a5), a6), a7), a8), a| (a1, a2, a3, a4, a5, a6, a7, a8, a)
        )
    }
}

impl<A> FromRequest for (A,)
where
    A: FromRequest,
    A::Error: IntoResponse,
{
    type Error = Response;

    type Future = Fr<FrpCall<()>, (), A, fn((),A) -> (A,)>;

    fn from_request(req: Request) -> Self::Future {
        let (parts,body) = req.into_parts();
        fr(fc(parts), body, |_,a|(a,))
    }
}

impl<A1,A> FromRequest for (A1,A)
where
    A1: FromRequestParts,
    A1::Error: IntoResponse,
    A: FromRequest,
    A::Error: IntoResponse,
{
    type Error = Response;

    type Future = Fr<FrpCall<A1>, A1, A, fn(A1,A) -> (A1,A)>;

    fn from_request(req: Request) -> Self::Future {
        let (parts,body) = req.into_parts();
        fr(fc(parts), body, |a1,a|(a1,a))
    }
}

// ===== Futures Parts =====

// FrpCall  = <Frp>(Parts) -> (Parts,Frp)
// Frp      = <Frp2>(Parts,Frp1) -> (Parts,(Frp1,Frp2))
// Fr       = <Fr>(Parts,Frp) -> (Frp,Fr)

// <Frp>(Parts) -> (Parts,Frp)

pin_project_lite::pin_project! {
    /// Future that wrap FromRequestParts future.
    pub struct FrpCall<Frp>
    where
        Frp: FromRequestParts,
    {
        #[pin] f: Frp::Future,
        parts: Option<request::Parts>,
    }
}

fn fc<Frp>(mut parts: request::Parts) -> FrpCall<Frp>
where
    Frp: FromRequestParts,
{
    FrpCall {
        f: Frp::from_request_parts(&mut parts),
        parts: Some(parts),
    }
}

impl<Frp> Future for FrpCall<Frp>
where
    Frp: FromRequestParts,
    Frp::Error: IntoResponse,
{
    type Output = Result<(request::Parts, Frp), Response>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let me = self.project();
        match ready!(me.f.poll(cx)) {
            Ok(frp) => Ready(Ok((me.parts.take().unwrap(), frp))),
            Err(err) => Ready(Err(err.into_response())),
        }
    }
}

// <Frp2>(Parts,Frp1) -> (Parts,(Frp1,Frp2))

pin_project_lite::pin_project! {
    /// future that wrap subsequent FromRequestParts future
    #[project = FrpProj]
    pub enum Frp<Fut,Frp1,Frp2>
    where
        Frp2: FromRequestParts,
    {
        Frp1 { #[pin] f: Fut, },
        Frp2 { #[pin] f: Frp2::Future, parts: Option<request::Parts>, frp1: Option<Frp1>, },
    }
}

fn fp<Fut, Frp1, Frp2>(f: Fut) -> Frp<Fut, Frp1, Frp2>
where
    Frp2: FromRequestParts,
{
    Frp::Frp1 { f }
}

impl<Fut, Frp1, Frp2> Future for Frp<Fut, Frp1, Frp2>
where
    Fut: Future<Output = Result<(request::Parts, Frp1), Response>>,
    Frp2: FromRequestParts,
    Frp2::Error: IntoResponse,
{
    type Output = Result<(request::Parts, (Frp1, Frp2)), Response>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        loop {
            match self.as_mut().project() {
                FrpProj::Frp1 { f } => match ready!(f.poll(cx)) {
                    Ok((mut parts, frp1)) => self.set(Frp::Frp2 {
                        f: Frp2::from_request_parts(&mut parts),
                        frp1: Some(frp1),
                        parts: Some(parts),
                    }),
                    Err(err) => return Ready(Err(err)),
                },
                FrpProj::Frp2 { f, parts, frp1 } => {
                    return match ready!(f.poll(cx)) {
                        Ok(frp2) => {
                            Ready(Ok((parts.take().unwrap(), (frp1.take().unwrap(), frp2))))
                        }
                        Err(err) => Ready(Err(err.into_response())),
                    };
                }
            }
        }
    }
}

// <Fr>(Parts,Frp) -> (Frp,Fr)

pin_project_lite::pin_project! {
    /// future that wrap subsequent FromRequest future
    #[project = FrProj]
    pub enum Fr<Fut,Frp1,Fr1,M>
    where
        Fr1: FromRequest,
    {
        Frp { #[pin] f: Fut, body: Option<Body>, m: Option<M> },
        Fr { #[pin] f: Fr1::Future, frp: Option<Frp1>, m: Option<M> },
    }
}

fn fr<Fut, Frp1, Fr1, M>(f: Fut, body: Body, m: M) -> Fr<Fut, Frp1, Fr1, M>
where
    Fut: Future<Output = Result<(request::Parts, Frp1), Response>>,
    Fr1: FromRequest,
{
    Fr::Frp {
        f,
        body: Some(body),
        m: Some(m),
    }
}

impl<Fut, Frp1, Fr1, M, M1> Future for Fr<Fut, Frp1, Fr1, M>
where
    Fut: Future<Output = Result<(request::Parts, Frp1), Response>>,
    Fr1: FromRequest,
    Fr1::Error: IntoResponse,
    M: FnOnce(Frp1,Fr1) -> M1,
{
    type Output = Result<M1, Response>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        loop {
            match self.as_mut().project() {
                FrProj::Frp { f, body, m } => match ready!(f.poll(cx)) {
                    Ok((parts, frp)) => {
                        let req = Request::from_parts(parts, body.take().unwrap());
                        let m = m.take();
                        self.set(Fr::Fr {
                            f: Fr1::from_request(req),
                            frp: Some(frp),
                            m,
                        })
                    }
                    Err(err) => return Ready(Err(err)),
                },
                FrProj::Fr { f, frp, m } => {
                    return match ready!(f.poll(cx)) {
                        Ok(fr) => Ready(Ok(m.take().unwrap()(frp.take().unwrap(), fr))),
                        Err(err) => Ready(Err(err.into_response())),
                    };
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::request::FromRequest;

    use http::Method as M;

    type S = String;

    #[test]
    fn assert_tuple() {
        let _f = <(M,) as FromRequest>::from_request;
        let _f = <(S,) as FromRequest>::from_request;
        let _f = <(M,S,) as FromRequest>::from_request;
        let _f = <(M,M,S,) as FromRequest>::from_request;
        let _f = <(M,M,M,S,) as FromRequest>::from_request;
        let _f = <(M,M,M,M,S,) as FromRequest>::from_request;
        let _f = <(M,M,M,M,M,S,) as FromRequest>::from_request;
        let _f = <(M,M,M,M,M,M,S,) as FromRequest>::from_request;
        let _f = <(M,M,M,M,M,M,M,S,) as FromRequest>::from_request;
        let _f = <(M,M,M,M,M,M,M,M,S,) as FromRequest>::from_request;
    }
}


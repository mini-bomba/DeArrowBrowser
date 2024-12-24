/* This file is part of the DeArrow Browser project - https://github.com/mini-bomba/DeArrowBrowser
*
*  Copyright (C) 2024 mini_bomba
*  
*  This program is free software: you can redistribute it and/or modify
*  it under the terms of the GNU Affero General Public License as published by
*  the Free Software Foundation, either version 3 of the License, or
*  (at your option) any later version.
*
*  This program is distributed in the hope that it will be useful,
*  but WITHOUT ANY WARRANTY; without even the implied warranty of
*  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
*  GNU Affero General Public License for more details.
*
*  You should have received a copy of the GNU Affero General Public License
*  along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

use std::future::{ready, Ready};

use actix_web::body::{BoxBody, EitherBody, MessageBody};
use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::error::Error;
use actix_web::http::header::{Accept, ContentType, Header};
use cloneable_errors::SerializableError;
use futures::{future::LocalBoxFuture, FutureExt};
use log::warn;

use crate::utils::{HeaderMapExt, SerializableErrorResponseMarker};


pub struct ErrorRepresentation;

impl<S, B> Transform<S, ServiceRequest> for ErrorRepresentation
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<B, BoxBody>>;
    type Error = Error;
    type InitError = ();
    type Transform = ErrorRepresentationInstance<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(ErrorRepresentationInstance { service }))
    }
}

pub struct ErrorRepresentationInstance<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for ErrorRepresentationInstance<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<B, BoxBody>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let requested_json = Accept::parse(&req).is_ok_and(|a| a.iter().any(|item| item.item.essence_str() == "application/json"));

        let srv = self.service.call(req);

        async move {
            let resp = srv.await?;
            if requested_json || !resp.response().extensions().contains::<SerializableErrorResponseMarker>() {
                return Ok(resp.map_into_left_body())
            }

            // client did not explicitly request json and the response contains serialized error
            // json - convert to plaintext
            let resp = resp.map_body(|head, body| {
                match body.try_into_bytes() {
                    Err(body) => {
                        warn!("Failed to read & convert the body of a SerializableError response");
                        EitherBody::left(body)
                    },
                    Ok(bytes) => {
                        match serde_json::from_slice::<SerializableError>(&bytes) {
                            Err(err) => {
                                warn!("Failed to deserialize the SerializableError response: {err}");
                                EitherBody::right(BoxBody::new(bytes))
                            },
                            Ok(error) => {
                                if let Err(err) = head.headers.replace_header(ContentType::plaintext()) {
                                    warn!("Failed to replace the ContentType header: {err}");
                                }
                                EitherBody::right(BoxBody::new(format!("{error:?}")))
                            }
                        }
                    }
                }
            });
            Ok(resp)
        }.boxed_local()
    }
}

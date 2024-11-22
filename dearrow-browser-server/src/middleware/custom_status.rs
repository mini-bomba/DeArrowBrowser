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

use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::error::Error;
use futures::{future::LocalBoxFuture, FutureExt};


pub struct CustomStatusCodes;


impl<S, B> Transform<S, ServiceRequest> for CustomStatusCodes
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = CustomStatusCodesInstance<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(CustomStatusCodesInstance { service }))
    }
}

pub struct CustomStatusCodesInstance<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for CustomStatusCodesInstance<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let srv = self.service.call(req);
        
        async move {
            let mut resp = srv.await?;
            let head = resp.response_mut().head_mut();

            #[allow(clippy::single_match)] // we may have more codes later
            match head.status.as_u16() {
                333 => {
                    head.reason = Some("Not Ready Yet");
                }
                _ => {},
            }

            Ok(resp)
        }.boxed_local()
    }
}

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

use actix_web::{body::{BoxBody, EitherBody}, dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform}, http::{header::{CacheControl, CacheDirective, ETag, Header, IfNoneMatch}, StatusCode}, Error, HttpResponseBuilder};
use futures::{future::LocalBoxFuture, FutureExt};

use crate::constants::DB_READ_ERR;
use crate::state::DBLock;
use crate::utils::{self, HeaderMapExt};

pub struct ETagCache;

impl<S, B> Transform<S, ServiceRequest> for ETagCache
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B, BoxBody>>;
    type Error = Error;
    type InitError = ();
    type Transform = ETagCacheInstance<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(ETagCacheInstance { service }))
    }
}

pub struct ETagCacheInstance<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for ETagCacheInstance<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B, BoxBody>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let db = req.app_data::<DBLock>().unwrap().clone();
        let Ok(etag) = db.read().map(|db| db.get_etag()) else {
            return ready(Err(utils::Error::from(DB_READ_ERR.clone()).into())).boxed_local();
        };

        let inm = match IfNoneMatch::parse(&req) {
            Ok(inm) => inm,
            Err(err) => return ready(Err(err.into())).boxed_local(),
        };

        if let IfNoneMatch::Items(etags) = inm {
            if etags.iter().any(|e| e.weak_eq(&etag)) {
                let mut resp = HttpResponseBuilder::new(StatusCode::NOT_MODIFIED);
                resp.append_header(ETag(etag))
                    .append_header(CacheControl(vec![CacheDirective::NoCache]));
                return ready(Ok(req.into_response(resp).map_into_right_body())).boxed_local();
            }
        }

        let srv = self.service.call(req);

        async move {
            let mut resp = srv.await?;

            let headers = resp.headers_mut();
            headers.append_header(ETag(etag)).map_err(Into::<actix_web::error::HttpError>::into)?;
            headers.append_header(CacheControl(vec![CacheDirective::NoCache])).map_err(Into::<actix_web::error::HttpError>::into)?;

            Ok(resp.map_into_left_body())
        }.boxed_local()
    }
}

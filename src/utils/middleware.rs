use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    error::ErrorUnauthorized,
    Error, HttpMessage,
};
use futures::future::{ready, LocalBoxFuture, Ready};
use std::rc::Rc;

use crate::auth::jwt::{validate_token, Claims};
use crate::models::user::UserRole;

pub struct Authentication {
    jwt_secret: String,
}

impl Authentication {
    pub fn new(jwt_secret: String) -> Self {
        Self { jwt_secret }
    }
}

impl<S, B> Transform<S, ServiceRequest> for Authentication
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = AuthenticationMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthenticationMiddleware {
            service: Rc::new(service),
            jwt_secret: self.jwt_secret.clone(),
        }))
    }
}

pub struct AuthenticationMiddleware<S> {
    service: Rc<S>,
    jwt_secret: String,
}

impl<S, B> Service<ServiceRequest> for AuthenticationMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // Skip authentication for login endpoint
        if req.path() == "/api/auth/login" {
            let fut = self.service.call(req);
            return Box::pin(async move {
                let res = fut.await?;
                Ok(res)
            });
        }

        let auth_header = req.headers().get("Authorization").cloned();
        let jwt_secret = self.jwt_secret.clone();
        let service = Rc::clone(&self.service);

        Box::pin(async move {
            if let Some(auth_value) = auth_header {
                let auth_str = auth_value.to_str().map_err(|_| ErrorUnauthorized("Invalid authorization header"))?;
                
                if auth_str.starts_with("Bearer ") {
                    let token = &auth_str[7..]; // Remove "Bearer " prefix
                    
                    match validate_token(token, jwt_secret.as_bytes()) {
                        Ok(claims) => {
                            // Add claims to request extensions
                            req.extensions_mut().insert(claims);
                            let fut = service.call(req);
                            let res = fut.await?;
                            return Ok(res);
                        }
                        Err(_) => {
                            return Err(ErrorUnauthorized("Invalid token"));
                        }
                    }
                }
            }
            
            Err(ErrorUnauthorized("Authorization header missing"))
        })
    }
}

pub struct RoleAuthorization {
    allowed_roles: Vec<UserRole>,
}

impl RoleAuthorization {
    pub fn new(allowed_roles: Vec<UserRole>) -> Self {
        Self { allowed_roles }
    }
}

impl<S, B> Transform<S, ServiceRequest> for RoleAuthorization
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = RoleAuthorizationMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RoleAuthorizationMiddleware {
            service: Rc::new(service),
            allowed_roles: self.allowed_roles.clone(),
        }))
    }
}

pub struct RoleAuthorizationMiddleware<S> {
    service: Rc<S>,
    allowed_roles: Vec<UserRole>,
}

impl<S, B> Service<ServiceRequest> for RoleAuthorizationMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = Rc::clone(&self.service);
        let allowed_roles = self.allowed_roles.clone();

        Box::pin(async move {
            // Get claims from extensions
            let claims_opt = req.extensions().get::<Claims>().cloned();
            
            if let Some(claims) = claims_opt {
                let user_role = UserRole::from_str(&claims.role).unwrap_or(UserRole::User);
                
                let is_allowed = allowed_roles.iter().any(|role| {
                    match (role, &user_role) {
                        (UserRole::Admin, _) => false, // Admin role in allowed_roles doesn't automatically grant access
                        (_, UserRole::Admin) => true,  // User with Admin role can access anything
                        (role1, role2) => std::mem::discriminant(role1) == std::mem::discriminant(role2),
                    }
                });
                
                if is_allowed {
                    let fut = service.call(req);
                    let res = fut.await?;
                    return Ok(res);
                }
                
                return Err(ErrorUnauthorized("Insufficient permissions"));
            }
            
            Err(ErrorUnauthorized("Authentication required"))
        })
    }
}
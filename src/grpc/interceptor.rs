use tonic::{Request, Status};

#[derive(Debug, Clone)]
pub struct AuthInterceptor;

impl tonic::service::Interceptor for AuthInterceptor {
    fn call(&mut self, mut request: Request<()>) -> Result<Request<()>, Status> {
        let token = request
            .metadata()
            .get("authorization")
            .and_then(|m| m.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "))
            .map(|s| s.to_string());

        match token {
            Some(t) => match crate::jwt::decode_token(&t) {
                Ok(claims) => {
                    request.extensions_mut().insert(claims);
                    Ok(request)
                }
                Err(e) => Err(Status::unauthenticated(format!("Invalid token: {}", e))),
            },
            None => Err(Status::unauthenticated("Missing authorization token")),
        }
    }
}

use std::time::Duration;

pub trait ReqwestClientProvider: Send + Sync {
    fn client(&self) -> Result<reqwest::Client, reqwest::Error>;
}

#[derive(Clone)]
pub struct DefaultReqwestClientProvider;

impl ReqwestClientProvider for DefaultReqwestClientProvider {
    fn client(&self) -> Result<reqwest::Client, reqwest::Error> {
        reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(20))
            .read_timeout(Duration::from_secs(8))
            .build()
    }
}

pub mod facebook;

pub struct Slack {
    pub token: String,
}

pub struct Facebook {
    pub token: String,
    pub secret: String,
}

pub struct Chat {
    slack: Slack,
    fb: Facebook,
}

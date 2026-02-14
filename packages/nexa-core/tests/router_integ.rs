use nexa_router::*;

#[derive(Clone, PartialEq, Default, Debug)]
enum TestRoute {
    #[default]
    Home,
    User(String),
}

impl std::fmt::Display for TestRoute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestRoute::Home => write!(f, "/"),
            TestRoute::User(id) => write!(f, "/user/{}", id),
        }
    }
}

impl Routable for TestRoute {
    fn from_path(path: &str) -> Option<Self> {
        if path == "/" {
            Some(TestRoute::Home)
        } else if path.starts_with("/user/") {
            Some(TestRoute::User(path[6..].to_string()))
        } else {
            None
        }
    }
}

#[test]
fn test_router_matching_integration() {
    let nav = Navigator::<TestRoute>::new();
    nav.push(TestRoute::User("123".to_string()));
    assert_eq!(nav.current(), TestRoute::User("123".to_string()));
}

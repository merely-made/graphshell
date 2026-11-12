use std::sync::{Arc, Mutex};

use super::*;
use chirograph::ProjectionSession;

#[derive(Default)]
struct FixtureEndpoint {
    polls: Arc<Mutex<u32>>,
}

impl ProjectionCatalog for FixtureEndpoint {
    fn describe(&self) -> EndpointDescriptor {
        EndpointDescriptor {
            label: "Fixture endpoint".to_string(),
            projections: Vec::new(),
        }
    }
}

impl ProjectionSource for FixtureEndpoint {
    type Error = String;

    fn snapshot(&mut self, _: ProjectionRequest) -> Result<ProjectionSnapshot, Self::Error> {
        Err("fixture has no snapshot".to_string())
    }
}

impl PresentationSource for FixtureEndpoint {
    type Error = String;

    fn resource(&mut self, _: ResourceRequest) -> Result<ResourceResponse, Self::Error> {
        Err("fixture has no resource".to_string())
    }
}

impl IntentSink for FixtureEndpoint {
    type Error = String;

    fn invoke(&mut self, _: IntentInvocation) -> Result<IntentResult, Self::Error> {
        Err("fixture has no intent".to_string())
    }
}

impl ProjectionNoticeSource for FixtureEndpoint {
    type Error = String;

    fn poll_notice(&mut self) -> Result<Option<CarrierNotice>, Self::Error> {
        *self.polls.lock().unwrap() += 1;
        Ok(None)
    }
}

#[test]
fn catalog_binds_one_named_notifying_endpoint_to_the_admitted_context() {
    let context = AdmittedEndpointContext::new(
        ProjectionSession("admitted:catalog".to_string()),
        [0xC0; 32],
    );
    let seen = Arc::new(Mutex::new(None));
    let polls = Arc::new(Mutex::new(0));
    let mut catalog = ResidentEndpointCatalog::new();
    let factory_seen = Arc::clone(&seen);
    let factory_polls = Arc::clone(&polls);
    catalog
        .register_notifying("cleromancy", "Local Cleromancy readings", move |context| {
            *factory_seen.lock().unwrap() = Some(context.clone());
            Ok(FixtureEndpoint {
                polls: Arc::clone(&factory_polls),
            })
        })
        .unwrap();

    assert_eq!(
        catalog.offers(),
        vec![ResidentEndpointOffer {
            id: "cleromancy".to_string(),
            label: "Local Cleromancy readings".to_string(),
        }]
    );
    let mut endpoint = catalog.open("cleromancy", &context).unwrap();
    assert_eq!(*seen.lock().unwrap(), Some(context));
    assert_eq!(endpoint.describe().label, "Fixture endpoint");
    assert_eq!(endpoint.poll_notice().unwrap(), None);
    assert_eq!(*polls.lock().unwrap(), 1);
}

#[test]
fn session_exposes_preparation_and_liveness_of_its_endpoint() {
    struct Stopped;
    impl ResidentEndpoint for Stopped {
        fn describe(&self) -> EndpointDescriptor {
            EndpointDescriptor {
                label: "stopped".into(),
                projections: vec![],
            }
        }
        fn snapshot(&mut self, _: ProjectionRequest) -> Result<ProjectionSnapshot, String> {
            Err("stopped".into())
        }
        fn resource(&mut self, _: ResourceRequest) -> Result<ResourceResponse, String> {
            Err("stopped".into())
        }
        fn invoke(&mut self, _: IntentInvocation) -> Result<IntentResult, String> {
            Err("stopped".into())
        }
        fn prepare(&mut self) -> Result<(), String> {
            Err("stopped".into())
        }
        fn is_live(&self) -> bool {
            false
        }
        fn requires_blocking(&self) -> bool {
            true
        }
    }
    let mut session = ResidentEndpointSession {
        endpoint: Box::new(Stopped),
    };
    assert!(session.requires_blocking());
    assert!(!session.is_live());
    assert!(session.prepare().is_err());
}

#[test]
fn removal_requires_the_registration_owner() {
    let mut catalog = ResidentEndpointCatalog::new();
    catalog
        .register_owned("fixture", "Fixture", 7, |_| {
            Ok(Box::new(TypedResidentEndpoint::silent(
                FixtureEndpoint::default(),
            )))
        })
        .unwrap();
    assert!(!catalog.remove_owned("fixture", 6));
    assert_eq!(catalog.offers().len(), 1);
    assert!(catalog.remove_owned("fixture", 7));
    assert!(catalog.offers().is_empty());
    catalog
        .register_owned("fixture", "Replacement", 8, |_| {
            Ok(Box::new(TypedResidentEndpoint::silent(
                FixtureEndpoint::default(),
            )))
        })
        .unwrap();
    assert!(!catalog.remove_owned("fixture", 7));
    assert_eq!(catalog.offers()[0].label, "Replacement");
}

#[test]
fn catalog_refuses_ambiguous_or_unknown_routes() {
    let mut catalog = ResidentEndpointCatalog::new();
    catalog
        .register("identity", "Identity", |_| Ok(FixtureEndpoint::default()))
        .unwrap();
    assert!(matches!(
        catalog.register("identity", "Duplicate", |_| Ok(FixtureEndpoint::default())),
        Err(ResidentEndpointCatalogError::Duplicate { .. })
    ));
    assert!(matches!(
        catalog.register("not a route", "Invalid", |_| Ok(FixtureEndpoint::default())),
        Err(ResidentEndpointCatalogError::InvalidId { .. })
    ));
    assert!(matches!(
        catalog.open(
            "missing",
            &AdmittedEndpointContext::new(
                ProjectionSession("admitted:missing".to_string()),
                [0; 32]
            ),
        ),
        Err(ResidentEndpointCatalogError::Unknown { .. })
    ));
}

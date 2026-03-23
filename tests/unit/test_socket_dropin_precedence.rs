use core_ops::core::validation::validate_socket_dropin_precedence;
use core_ops::core::types::DropInSource;

#[test]
fn rejects_host_socket_dropins_that_sort_before_base() {
    let base = vec![DropInSource {
        target: "alpha.socket".to_string(),
        contents: "BASE".to_string(),
        source_path: "/services/alpha/alpha.socket.d/10-defaults.conf".to_string(),
    }];
    let host = vec![DropInSource {
        target: "alpha.socket".to_string(),
        contents: "HOST".to_string(),
        source_path: "/hosts/kadath/overrides/alpha.socket.d/05-host.conf".to_string(),
    }];

    let err = validate_socket_dropin_precedence(&base, &host).expect_err("should fail");
    assert!(err
        .message
        .contains("host socket drop-in must sort after base drop-ins"));
}

#[test]
fn accepts_host_socket_dropins_that_sort_after_base() {
    let base = vec![DropInSource {
        target: "alpha.socket".to_string(),
        contents: "BASE".to_string(),
        source_path: "/services/alpha/alpha.socket.d/10-defaults.conf".to_string(),
    }];
    let host = vec![DropInSource {
        target: "alpha.socket".to_string(),
        contents: "HOST".to_string(),
        source_path: "/hosts/kadath/overrides/alpha.socket.d/90-host.conf".to_string(),
    }];

    validate_socket_dropin_precedence(&base, &host).expect("should pass");
}

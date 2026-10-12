//! Mandatory real-runtime checks, enabled explicitly by the acceptance runner.

use super::*;

fn required(name: &str) -> std::path::PathBuf {
    std::env::var_os(name).map(Into::into).unwrap_or_else(|| {
        panic!("{name} is required; run conformance/run.py with an independent SDK")
    })
}

fn rejected(host: &ManagedHost, id: i64, opcode: i32, expected: i32) {
    let mut output = 911_i64;
    // SAFETY: reviewed bridge, valid aligned storage, and integer-only inputs.
    assert_eq!(
        unsafe { (host.invoke)(id, opcode, 1, &mut output) },
        expected
    );
    assert_eq!(output, 911, "an error must not commit an output");
}

#[test]
fn real_runtime_callbacks_lifetime_errors_and_exception_containment() {
    let root = required("OU_HOST_DOTNET_ROOT");
    let fxr = required("OU_HOST_FXR");
    let bridge = required("OU_HOST_BRIDGE");
    let config = required("OU_HOST_CONFIG");
    let fixture = required("OU_HOST_SCRIPT");
    // SAFETY: acceptance runner supplies the recorded official SDK and reviewed bridge.
    let host = unsafe { ManagedHost::load(&root, &fxr, &bridge, &config) }
        .expect("real .NET host must initialize");
    let name = "OpenUnity.ManagedHostFixture.DeterministicScript";
    let mut first = host
        .create_script(&fixture, name)
        .expect("fixture loads with shared IHostedScript identity");
    let mut second = host.create_script(&fixture, name).unwrap();
    let first_id = first.id.unwrap();
    let second_id = second.id.unwrap();
    assert!(first_id > 0 && second_id > first_id);
    rejected(&host, first_id, 2, -5);
    rejected(&host, first_id, 3, -5);
    rejected(&host, first_id, 99, -2);
    rejected(&host, 0, 1, -1);
    // SAFETY: null is explicitly rejected by the bridge before dereferencing output.
    assert_eq!(
        unsafe { (host.invoke)(first_id, 1, 1, ptr::null_mut()) },
        -2
    );
    // SAFETY: null input is explicitly rejected before either pointer is read.
    assert_eq!(unsafe { (host.create)(ptr::null(), ptr::null()) }, -2);
    let path = CString::new(fixture.to_str().unwrap()).unwrap();
    let script_type = CString::new(name).unwrap();
    // SAFETY: valid terminated buffers; the bridge must reject each null
    // argument independently before accessing it.
    assert_eq!(
        unsafe { (host.create)(path.as_ptr().cast(), ptr::null()) },
        -2
    );
    // SAFETY: valid type buffer and a null path that the bridge must reject.
    assert_eq!(
        unsafe { (host.create)(ptr::null(), script_type.as_ptr().cast()) },
        -2
    );
    let missing_path = CString::new(fixture.with_extension("missing").to_str().unwrap()).unwrap();
    // SAFETY: valid terminated UTF-8 buffers. This reaches managed path
    // validation directly, bypassing the Rust wrapper's filesystem check.
    assert_eq!(
        unsafe { (host.create)(missing_path.as_ptr().cast(), script_type.as_ptr().cast()) },
        -2
    );
    let invalid_utf8 = [0xff_u8, 0];
    // SAFETY: both buffers are valid and NUL terminated; invalid UTF-8 must be rejected.
    assert_eq!(
        unsafe { (host.create)(path.as_ptr().cast(), invalid_utf8.as_ptr()) },
        -2
    );
    assert_eq!(first.initialize(10).unwrap(), 10);
    assert_eq!(second.initialize(100).unwrap(), 100);
    rejected(&host, first_id, 1, -5);
    assert_eq!(first.tick(3).unwrap(), 13);
    assert_eq!(second.tick(7).unwrap(), 107);
    assert_eq!(first.tick(-2).unwrap(), 11);
    assert_eq!(first.shutdown(4).unwrap(), 15);
    rejected(&host, first_id, 2, -5);
    rejected(&host, first_id, 3, -5);
    first.close().unwrap();
    rejected(&host, first_id, 1, -1);
    // SAFETY: stale integer handles must be rejected without touching managed storage.
    assert_eq!(unsafe { (host.destroy)(first_id) }, -1);
    drop(second);
    rejected(&host, second_id, 2, -1);

    let throwing = host
        .create_script(&fixture, "OpenUnity.ManagedHostFixture.ThrowingScript")
        .unwrap();
    let throwing_id = throwing.id.unwrap();
    rejected(&host, throwing_id, 1, -6);
    rejected(&host, throwing_id, 1, -5);
    rejected(&host, throwing_id, 2, -5);
    rejected(&host, throwing_id, 99, -2);
    throwing.close().unwrap();
    rejected(&host, throwing_id, 1, -1);
    let mut overflow = host.create_script(&fixture, name).unwrap();
    overflow.initialize(i64::MAX).unwrap();
    rejected(&host, overflow.id.unwrap(), 2, -6);
    rejected(&host, overflow.id.unwrap(), 3, -5);
    overflow.close().unwrap();
    let mut shutdown_overflow = host.create_script(&fixture, name).unwrap();
    shutdown_overflow.initialize(i64::MAX).unwrap();
    let mut output = 911_i64;
    // SAFETY: reviewed bridge, live session and aligned output; checked overflow
    // in the fixture must be contained even when the failing callback is Shutdown.
    assert_eq!(
        unsafe { (host.invoke)(shutdown_overflow.id.unwrap(), 3, 1, &mut output) },
        -6
    );
    assert_eq!(output, 911);
    rejected(&host, shutdown_overflow.id.unwrap(), 2, -5);
    shutdown_overflow.close().unwrap();
    for bad_type in [
        "No.Such.Type",
        "OpenUnity.ManagedHostFixture.NoDefaultConstructorScript",
    ] {
        assert_eq!(
            host.create_script(&fixture, bad_type).err().unwrap().code(),
            Some(-2)
        );
    }
    assert_eq!(
        host.create_script(
            &fixture,
            "OpenUnity.ManagedHostFixture.ThrowingConstructorScript"
        )
        .err()
        .unwrap()
        .code(),
        Some(-6)
    );
    assert!(host.create_script(&fixture, "bad\0type").is_err());
    let missing = fixture.with_extension("missing");
    assert!(host.create_script(&missing, name).is_err());
    // SAFETY: same trusted runtime and bridge; exercises already-initialized context handling.
    let another = unsafe { ManagedHost::load(&root, &fxr, &bridge, &config) }.unwrap();
    let fresh = another.create_script(&fixture, name).unwrap();
    assert!(
        fresh.id.unwrap() > throwing_id,
        "destroyed IDs cannot be recycled across host contexts"
    );
    fresh.close().unwrap();
}

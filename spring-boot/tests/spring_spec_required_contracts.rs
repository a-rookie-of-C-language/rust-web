use spring_boot::{Application, ApplicationContext, Component};

#[derive(Debug, Default, Clone)]
struct SpecMissingDep;

#[Component]
#[derive(Debug, Default, Clone)]
struct SpecNeedsMissingDep {
    #[autowired]
    dep: SpecMissingDep,
}

#[Component]
#[derive(Debug, Default, Clone)]
struct SpecInvalidValueConfig {
    #[Value("${spec.invalid.int:not-a-number}")]
    should_be_int: i32,
}

fn _touch_contract_fields(a: &SpecNeedsMissingDep, b: &SpecInvalidValueConfig) {
    let _ = &a.dep;
    let _ = b.should_be_int;
}

#[test]
fn required_contract_missing_autowired_dependency_rejects_bean() {
    let context = Application::run();
    if let (Some(a), Some(b)) = (
        context
            .get_bean("specNeedsMissingDep")
            .and_then(|x| x.downcast_ref::<SpecNeedsMissingDep>()),
        context
            .get_bean("specInvalidValueConfig")
            .and_then(|x| x.downcast_ref::<SpecInvalidValueConfig>()),
    ) {
        _touch_contract_fields(a, b);
    }
    assert!(
        context.get_bean("specNeedsMissingDep").is_none(),
        "bean creation should fail when required autowired dependency is absent"
    );
}

#[test]
fn required_contract_invalid_value_parse_rejects_bean() {
    let context = Application::run();
    assert!(
        context.get_bean("specInvalidValueConfig").is_none(),
        "bean creation should fail when @Value cannot parse target type"
    );
}

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
    let a = context
        .get_bean("specNeedsMissingDep")
        .and_then(|x| x.as_ref().downcast_ref::<SpecNeedsMissingDep>().cloned());
    let b = context
        .get_bean("specInvalidValueConfig")
        .and_then(|x| x.as_ref().downcast_ref::<SpecInvalidValueConfig>().cloned());
    if let (Some(a), Some(b)) = (a.as_ref(), b.as_ref()) {
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

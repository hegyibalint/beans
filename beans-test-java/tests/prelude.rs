use beans_test_harness::fixture::Fixture;
use beans_lang_java::JavaLanguage;

pub fn fixture() -> Fixture {
    Fixture::new()
        .with_language(JavaLanguage)
}

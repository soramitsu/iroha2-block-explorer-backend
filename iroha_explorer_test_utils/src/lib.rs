use serde::Serialize;

pub trait ExpectExt {
    fn assert_json_eq(&self, actual: impl Serialize);
}

impl ExpectExt for expect_test::Expect {
    fn assert_json_eq(&self, actual: impl Serialize) {
        let json = serde_json::to_string_pretty(&actual).unwrap();
        self.assert_eq(&json);
    }
}

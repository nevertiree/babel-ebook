use babel_ebook::escape::{html_escape, xml_escape};

#[test]
fn xml_escape_replaces_special_characters() {
    assert_eq!(
        xml_escape(r#"Tom & Jerry <The "Cat"> 'Mouse'"#),
        "Tom &amp; Jerry &lt;The &quot;Cat&quot;&gt; &apos;Mouse&apos;"
    );
}

#[test]
fn html_escape_is_alias_for_xml_escape() {
    assert_eq!(
        html_escape("<script>alert(1)</script>"),
        xml_escape("<script>alert(1)</script>")
    );
}

#[test]
fn escape_does_not_touch_plain_text() {
    assert_eq!(xml_escape("plain text"), "plain text");
}

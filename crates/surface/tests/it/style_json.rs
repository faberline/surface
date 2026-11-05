//! Pins the `Style` JSON bytes for styles that use only the fields that
//! existed at 8b4e205. The literals were written from that commit's field
//! order. New fields must not appear for such styles.

use surface::{Color, Length, Style};

#[test]
fn default_style_json_is_the_legacy_document() {
    let json = serde_json::to_string(&Style::default()).unwrap();
    assert_eq!(
        json,
        concat!(
            r#"{"width":"Auto","height":"Auto","min_width":0.0,"min_height":0.0,"#,
            r#""max_width":"Auto","max_height":"Auto","direction":"Column","#,
            r#""align":"Stretch","justify":"Start","position":"Flow","#,
            r#""inset":{"left":"Auto","top":"Auto","right":"Auto","bottom":"Auto"},"#,
            r#""gap":0.0,"padding":0.0,"grow":0.0,"wrap":false,"clip":false,"#,
            r#""scroll_x":0.0,"scroll_y":0.0,"z_index":0,"background":[0,0,0,0],"#,
            r#""foreground":[29,27,32,255],"radius":0.0,"font_size":16.0,"#,
            r#""border_width":0.0,"border_color":[0,0,0,0],"state_layer":[0,0,0,0]}"#,
        )
    );
}

#[test]
fn legacy_styled_json_keeps_the_legacy_keys() {
    let style = Style::row()
        .width(Length::Px(10.))
        .background(Color::rgb(1, 2, 3))
        .radius(4.);
    let json = serde_json::to_string(&style).unwrap();
    assert_eq!(
        json,
        concat!(
            r#"{"width":{"Px":10.0},"height":"Auto","min_width":0.0,"min_height":0.0,"#,
            r#""max_width":"Auto","max_height":"Auto","direction":"Row","#,
            r#""align":"Stretch","justify":"Start","position":"Flow","#,
            r#""inset":{"left":"Auto","top":"Auto","right":"Auto","bottom":"Auto"},"#,
            r#""gap":0.0,"padding":0.0,"grow":0.0,"wrap":false,"clip":false,"#,
            r#""scroll_x":0.0,"scroll_y":0.0,"z_index":0,"background":[1,2,3,255],"#,
            r#""foreground":[29,27,32,255],"radius":4.0,"font_size":16.0,"#,
            r#""border_width":0.0,"border_color":[0,0,0,0],"state_layer":[0,0,0,0]}"#,
        )
    );
}

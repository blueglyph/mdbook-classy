#![cfg(test)]

use std::io::Cursor;
use crate::{book_preprocessing, Classy};

static INPUT_PRE: &str = r##"
[
  {
    "root": "root",
    "config": {
    },
    "renderer": "html",
    "mdbook_version": "0.5.4"
  },
  {
    "items": [
      {
        "Chapter": {
          "name": "Chapter 1",
          "content": "##;

static INPUT_POST: &str = r##",
          "number": [
            1
          ],
          "sub_items": [],
          "path": "chapter_1.md",
          "source_path": "chapter_1.md",
          "parent_names": []
        }
      }
    ]
  }
]
"##;


static TESTS: &[(&str, &str)] = &[
    // correctly applied
    (
        r##"# 1\n\n{:.a b}\nx **y**\n\n# 2"##,
        "# 1\n\n<div class=\"a b\">\n\nx **y**\n\n\n</div>\n\n# 2"
    ),

    // correctly rejected
    (
        r##"# 1\n\n{:a b}\nx **y**\n\n# 2"##,
        "# 1\n\n{:a b}\nx **y**\n\n# 2"
    ),
    (
        r##"# 1\n\n{:a b}c\nx **y**\n\n# 2"##,
        "# 1\n\n{:a b}c\nx **y**\n\n# 2"
    ),

    // doesn't work as expected:
    #[cfg(any())]
    (
        r##"# 1\n\n{:.a}\n\n> x **y**\n\n# 2"##,
        "# 1\n\n<div class=\"a\">\n\n# x **y**\n\n\n</div>\n\n# 2",
    ),
    #[cfg(any())]
    (
        r##"# 1\n\n{:.a}\n# x\n\n# 2"##,
        "# 1\n\n<div class=\"a b\">\n\n# x\n\n\n</div>\n\n# 2"
    ),
    #[cfg(any())]
    (
        r##"# 1\n\n{:.a}\n* x\n* y\n\n# 2"##,
        "# 1\n\n<div class=\"a b\">\n\n* x\n* y\n\n\n</div>\n\n# 2"
    ),
    #[cfg(any())]
    (
        r##"# 1\n\n{:.a}\n{:.b}\nx **y**\n\n# 2"##,
        "# 1\n\n<div class=\"a b\">\n\nx **y**\n\n\n</div>\n\n# 2"
    ),
];

#[test]
fn test_div() {
    const VERBOSE: bool = false;
    for (i, &(input, md_expected)) in TESTS.into_iter().enumerate() {
        if VERBOSE { println!("Test {i}: {input}"); }
        let preprocessor = Classy::new();
        let json = format!("{INPUT_PRE}\"{input}\"{INPUT_POST}");
        let stream = Cursor::new(json);
        let preprocessed = book_preprocessing(&preprocessor, stream).unwrap();
        let md_result = preprocessed.chapters().map(|c| c.content.clone()).collect::<Vec<_>>().join("\n");
        assert_eq!(md_result, md_expected);
    }
}

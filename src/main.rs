pub mod blogcard;
pub mod entity;
pub mod io;
pub mod parser;
pub mod template;
pub mod translator;
pub mod webpage;

use crate::translator::Translator;
use std::error::Error;
use std::path::Path;
use structopt::StructOpt;

use crate::entity::html::HtmlDoc;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(long = "debug")]
    pub debug: bool,
    #[structopt(long = "out", short = "o")]
    pub output: Option<String>,
    #[structopt(long = "standalone", short = "s")]
    pub standalone: bool,
    #[structopt(long = "compact", short = "c")]
    pub compact: bool,
    #[structopt(long = "indent", default_value = "2")]
    pub indent: usize,
    #[structopt(short = "H", long = "include-in-header")]
    pub include_in_header: Vec<String>,
    #[structopt(short = "B", long = "include-before-body")]
    pub include_before_body: Vec<String>,
    #[structopt(short = "A", long = "include-after-body")]
    pub include_after_body: Vec<String>,
    #[structopt(short = "C", long = "css")]
    pub css: Vec<String>,
    #[structopt(name = "input", default_value = "-")]
    pub input: Vec<String>,
}

fn eval(input: &String, debug: bool) -> Result<HtmlDoc, Box<dyn Error>> {
    if debug {
        eprintln!(">>> Reading {:?}", input);
    }
    let filedir: Option<String> = {
        Path::new(&input)
            .parent()
            .map(|path| String::from(path.to_str().unwrap()))
    };
    if debug {
        eprintln!(">>> filedir = {:?}", &filedir);
    }
    let content = io::read(&input)?;
    let mkd = parser::markdown(&content)?;
    if debug {
        eprintln!(">>> markdown = {:?}", &mkd);
    }
    let tr = Translator::new(filedir);
    let doc = tr.markdown(&mkd);
    if debug {
        eprintln!(">>> htmldoc = {:?}", &doc);
    }
    Ok(doc)
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();
    if opt.debug {
        eprintln!(">>> opt = {:?}", &opt);
    }

    // evaluating & flatten markdowns
    let mut doc = eval(&opt.input[0], opt.debug)?;
    for i in 1..opt.input.len() {
        let mut d = eval(&opt.input[i], opt.debug)?;
        doc.append(&mut d);
    }

    // show
    let body = doc.show(opt.compact, opt.indent);
    let html = if opt.standalone {
        let headers = io::reads(&opt.include_in_header)?;
        let befores = io::reads(&opt.include_before_body)?;
        let afters = io::reads(&opt.include_after_body)?;
        let ctx = template::Context::new(doc.title, body, opt.css, headers, befores, afters);
        template::simple(ctx)?
    } else {
        body
    };
    io::write(&opt.output, &html)?;
    Ok(())
}

#[cfg(test)]
mod test_main {

    use crate::parser;
    use crate::translator::Translator;

    macro_rules! assert_convert {
        ($compact:expr, $markdown:expr, $title:expr, $body:expr) => {
            let mkd = parser::markdown($markdown).unwrap();
            let tr = Translator::new(None);
            let doc = tr.markdown(&mkd);
            let title = doc.title.to_string();
            let body = doc.show($compact, 2);
            assert_eq!((title, body), (String::from($title), String::from($body)));
        };
        (compact; $markdown:expr, $title:expr, $body:expr) => {
            assert_convert!(true, $markdown, $title, $body)
        };
    }

    #[test]
    fn test_convert() {
        assert_convert!(compact; "# h1\n", "h1", "<h1>h1</h1>\n");
        assert_convert!(compact; "## h2\n", "h2", "<h2>h2</h2>\n");
        assert_convert!(compact; "a  b\nc\n", "a b c", "<p>a b c</p>\n");
        assert_convert!(compact; "a  \nb\nc\n\n---\n", "a b c", "<p>a <br /> b c</p><hr />\n");
        assert_convert!(compact; "*a* <!-- b -->\n",
            "a",
            "<p><em>a</em> <!-- b --></p>\n");
        assert_convert!(compact; "- a\n- b\n- c\n",
            "",
            "<ul><li>a</li><li>b</li><li>c</li></ul>\n"
        );
        assert_convert!(compact; "| A |\n|:-:|\n| a |\n",
            "",
            "<table><thead><tr class=header><th align=center>A</th></tr></thead><tbody><tr class=odd><td align=center>a</td></tr></tbody></table>\n"
        );
        assert_convert!(compact; "| A |\n| a |\n",
            "",
            "<table><tbody><tr class=odd><td align=left>A</td></tr><tr class=even><td align=left>a</td></tr></tbody></table>\n"
        );
        assert_convert!(compact; "[[http://example.com/]]\n",
            "http://example.com/",
            "<p><a href=\"http://example.com/\">Example Domain</a></p>\n"
        );
    }

    #[test]
    fn test_safe_encode() {
        assert_convert!(compact; "`<code>`\n", "&lt;code&gt;", "<p><code>&lt;code&gt;</code></p>\n");
    }

    #[test]
    fn test_raw_html() {
        assert_convert!(compact; "# test\n<div>Hi</div>\n", "test", "<h1>test</h1><p><div>Hi</div></p>\n");
    }

    #[test]
    fn test_link_block() {
        assert_convert!(compact;
            "# test\n{{ https://www.youtube.com/watch?v=_FKRL-t8aM8 }}\n",
            "test",
            "<h1>test</h1><div class=\"youtube\" src-id=\"_FKRL-t8aM8\"></div>\n"
        );
    }
}

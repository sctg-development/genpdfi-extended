use std::fs;
use std::path::PathBuf;

use genpdfi_extended::{fonts, elements, Document};

fn main() {
    if !cfg!(feature = "images") {
        eprintln!("This example requires the 'images' feature to be enabled.");
        return;
    }

    #[cfg(feature = "images")]
    {
        println!("Running example: SVG parser tests");

        // Prepare output directory
        let out_dir = PathBuf::from("examples/output");
        fs::create_dir_all(&out_dir).expect("create examples/output dir");

        // Load font family
        let font_data = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/fonts/NotoSans-Regular.ttf")).to_vec();
        let fd = fonts::FontData::new(font_data, None).expect("font data");
        let family = fonts::FontFamily { regular: fd.clone(), bold: fd.clone(), italic: fd.clone(), bold_italic: fd.clone() };

        // Create a document
        let mut doc = Document::new(family);
        doc.set_title("SVG Parser Tests");
        doc.push(elements::Paragraph::new("SVG Parser / printpdf diagnostics"));

        // Helper to attempt parsing (to collect warnings) and rendering
        fn try_parse_and_render(title: &str, svg: &str, doc: &mut Document) {
            doc.push(elements::Paragraph::new(""));
            doc.push(elements::Paragraph::new(title));

            // Attempt to parse with printpdf::Svg to get warnings
            let mut warnings = Vec::new();
            match printpdf::Svg::parse(svg, &mut warnings) {
                Ok(_svg_obj) => {
                    if !warnings.is_empty() {
                        doc.push(elements::Paragraph::new("Parser warnings:"));
                        for w in warnings.iter() {
                            doc.push(elements::Paragraph::new(format!("- {:?}", w)));
                        }
                    } else {
                        doc.push(elements::Paragraph::new("No parser warnings."));
                    }
                }
                Err(e) => {
                    doc.push(elements::Paragraph::new(format!("Parser error: {}", e)));
                }
            }

            // Try to render the SVG using our Image wrapper
            match elements::Image::from_svg_string(svg) {
                Ok(img) => {
                    doc.push(img);
                }
                Err(e) => {
                    doc.push(elements::Paragraph::new(format!("Image parsing/embedding failed: {}", e)));
                }
            }
        }

        // 1) Simple shapes (control)
        let svg_simple = r##"<svg width="200" height="100" xmlns="http://www.w3.org/2000/svg">
  <circle cx="50" cy="50" r="40" fill="#FF6B6B"/>
  <circle cx="150" cy="50" r="40" fill="#4ECDC4"/>
</svg>"##;
        try_parse_and_render("1. Simple shapes", svg_simple, &mut doc);

        // 2) Paths with arcs (A commands) - close to pie slice arcs
        let svg_arcs = r##"<svg width="220" height="120" xmlns="http://www.w3.org/2000/svg">
  <path d="M110 10 A100 100 0 0 1 210 110 L110 110 Z" fill="#ECECFF"/>
  <path d="M210 110 A100 100 0 0 1 110 210 L110 110 Z" fill="#FFFFDE"/>
</svg>"##;
        try_parse_and_render("2. Paths with arcs (A command)", svg_arcs, &mut doc);

        // 3) Style tag inside a group (some parsers ignore these)
        let svg_group_style = r##"<svg width="200" height="120" xmlns="http://www.w3.org/2000/svg">
  <g>
    <style>
      .f{fill:#ECECFF} .g{fill:#FFFFDE}
    </style>
    <rect x="10" y="10" width="80" height="40" class="f"/>
    <rect x="110" y="10" width="80" height="40" class="g"/>
  </g>
</svg>"##;
        try_parse_and_render("3. <style> inside <g>", svg_group_style, &mut doc);

        // 4) Gradient and defs
        let svg_defs = r##"<svg width="200" height="120" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <linearGradient id="g1">
      <stop offset="0%" stop-color="#FFD93D"/>
      <stop offset="100%" stop-color="#FF6B6B"/>
    </linearGradient>
  </defs>
  <rect x="10" y="10" width="180" height="100" fill="url(#g1)"/>
</svg>"##;
        try_parse_and_render("4. defs / gradients", svg_defs, &mut doc);

        // 5) viewBox + percentage width + style max-width & hsl colors (like Mermaid)
        let svg_mermaid_like = r##"<svg id="d" width="100%" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1093.9375 900" style="max-width:546.96875px;">
  <g transform="translate(225,225)">
    <circle cx="0" cy="0" r="186" fill="white" stroke="black"/>
    <path d="M0,-185A185,185,0,1,1,-177.899,-50.763L0,0Z" fill="#ECECFF"/>
    <path d="M-177.899,-50.763A185,185,0,0,1,-35.652,-181.532L0,0Z" fill="#ffffde"/>
    <path d="M-35.652,-181.532A185,185,0,0,1,0,-185L0,0Z" fill="hsl(80,100%,56.2745%)"/>
  </g>
</svg>"##;
        try_parse_and_render("5. Mermaid-like pie (viewBox, percent width, hsl colors)", svg_mermaid_like, &mut doc);

        // 6) Complex style blocks and many class rules (inline <style> at root)
        let svg_style_root = r##"<svg width="300" height="160" xmlns="http://www.w3.org/2000/svg">
  <style>
    .a{fill:#FFC9DE} .b{fill:#C9F2D1} .c{fill:#D1D1F2} .stroke{stroke:#000;stroke-width:2px}
  </style>
  <g transform="translate(40,20)">
    <rect x="0" y="0" width="80" height="60" class="a stroke"/>
    <rect x="100" y="0" width="80" height="60" class="b stroke"/>
    <rect x="200" y="0" width="80" height="60" class="c stroke"/>
  </g>
</svg>"##;
        try_parse_and_render("6. Root <style> with many rules", svg_style_root, &mut doc);

        // 7) An SVG with mask (should be stripped by our parser workaround)
        let svg_mask = r##"<svg width="200" height="120" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <mask id="m1">
      <rect x="0" y="0" width="200" height="120" fill="white"/>
      <circle cx="50" cy="50" r="30" fill="black"/>
    </mask>
  </defs>
  <rect x="0" y="0" width="200" height="120" fill="#95E1D3" mask="url(#m1)"/>
</svg>"##;
        try_parse_and_render("7. mask usage (test mask stripping)", svg_mask, &mut doc);

        // 8) Very long style rules and CSS features
        let svg_long_css = r##"<svg width="260" height="120" xmlns="http://www.w3.org/2000/svg">
  <style>
    .a{font-family: "Trebuchet MS", verdana, arial, sans-serif; font-size:14px; fill:#333}
    .b{opacity:0.8; stroke-dasharray:3 3; stroke-linecap:round}
  </style>
  <rect x="10" y="10" width="80" height="40" class="a b"/>
  <text x="10" y="90" class="a">Text sample</text>
</svg>"##;
        try_parse_and_render("8. Long CSS rules (font, opacity, dasharray)", svg_long_css, &mut doc);

        // 9) Pie from the log (full Mermaid sample) - included as-is to stress the parser
        let svg_mermaid_full = r##"<svg id="div" width="100%" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1093.9375 900" style="max-width: 546.96875px;" role="graphics-document document" aria-roledescription="pie">
  <g transform="scale(2)">
    <style>
      #div{font-family:"trebuchet ms",verdana,arial,sans-serif;font-size:16px;fill:#333;}@keyframes edge-animation-frame{from{stroke-dashoffset:0;}}@keyframes dash{to{stroke-dashoffset:0;}}#div .edge-animation-slow{stroke-dasharray:9,5!important;stroke-dashoffset:900;animation:dash 50s linear infinite;stroke-linecap:round;}#div .edge-animation-fast{stroke-dasharray:9,5!important;stroke-dashoffset:900;animation:dash 20s linear infinite;stroke-linecap:round;}#div .error-icon{fill:#552222;}#div .error-text{fill:#552222;stroke:#552222;}#div .edge-thickness-normal{stroke-width:1px;}#div .edge-thickness-thick{stroke-width:3.5px;}#div .edge-pattern-solid{stroke-dasharray:0;}#div .edge-thickness-invisible{stroke-width:0;fill:none;}#div .edge-pattern-dashed{stroke-dasharray:3;}#div .edge-pattern-dotted{stroke-dasharray:2;}#div .marker{fill:#333333;stroke:#333333;}#div .marker.cross{stroke:#333333;}#div svg{font-family:"trebuchet ms",verdana,arial,sans-serif;font-size:16px;}#div p{margin:0;}#div .pieCircle{stroke:black;stroke-width:2px;opacity:0.7;}#div .pieOuterCircle{stroke:black;stroke-width:2px;fill:none;}#div .pieTitleText{text-anchor:middle;font-size:25px;fill:black;font-family:"trebuchet ms",verdana,arial,sans-serif;}#div .slice{font-family:"trebuchet ms",verdana,arial,sans-serif;fill:#333;font-size:17px;}#div .legend text{fill:black;font-family:"trebuchet ms",verdana,arial,sans-serif;font-size:17px;}#div :root{--mermaid-font-family:"trebuchet ms",verdana,arial,sans-serif;}
    </style>
    <g></g>
    <g transform="translate(225,225)">
      <circle cx="0" cy="0" r="186" class="pieOuterCircle"></circle>
      <path d="M0,-185A185,185,0,1,1,-177.899,-50.763L0,0Z" fill="#ECECFF" class="pieCircle"></path>
      <path d="M-177.899,-50.763A185,185,0,0,1,-35.652,-181.532L0,0Z" fill="#ffffde" class="pieCircle"></path>
      <path d="M-35.652,-181.532A185,185,0,0,1,0,-185L0,0Z" fill="hsl(80, 100%, 56.2745098039%)" class="pieCircle"></path>
      <text transform="translate(83.57344705444068,110.75667676234512)" class="slice" style="text-anchor: middle;">79%</text>
      <text transform="translate(-93.90333582091016,-102.14561185731559)" class="slice" style="text-anchor: middle;">17%</text>
      <text transform="translate(-13.432508310066218,-138.09826291630174)" class="slice" style="text-anchor: middle;">3%</text>
      <text x="0" y="-200" class="pieTitleText">Pets adopted by volunteers</text>
      <g class="legend" transform="translate(216,-33)"><rect width="18" height="18" style="fill: rgb(236, 236, 255); stroke: rgb(236, 236, 255);"></rect><text x="22" y="14">Dogs</text></g>
      <g class="legend" transform="translate(216,-11)"><rect width="18" height="18" style="fill: rgb(255, 255, 222); stroke: rgb(255, 255, 222);"></rect><text x="22" y="14">Cats</text></g>
      <g class="legend" transform="translate(216,11)"><rect width="18" height="18" style="fill: rgb(181, 255, 32); stroke: rgb(181, 255, 32);"></rect><text x="22" y="14">Rats</text></g>
    </g>
  </g>
</svg>"##;
        try_parse_and_render("9. Full Mermaid sample (real-world stress test)", svg_mermaid_full, &mut doc);

        doc.push(elements::Paragraph::new(""));
        doc.push(elements::Paragraph::new("End of tests."));

        let mut pdf_file = fs::File::create(&out_dir.join("svg_parser_tests.pdf")).expect("create output file");
        doc.render(&mut pdf_file).expect("render document");
        println!("✓ Created examples/output/svg_parser_tests.pdf");
    }
}

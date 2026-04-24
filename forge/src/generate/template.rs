use super::util::esc;
pub(super) fn page_template(title: &str, main_content: &str, nav: &str, base: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<link rel="stylesheet" href="{base}assets/forge.css">
</head>
<body>
<div class="forge-layout">
{nav}
<main class="forge-main">
{main}
</main>
</div>
</body>
</html>
"#,
        title = esc(title),
        base = base,
        nav = nav,
        main = main_content,
    )
}

// ─── CSS Theme ───────────────────────────────────────────────────

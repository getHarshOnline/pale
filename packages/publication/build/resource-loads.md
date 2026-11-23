# Check resources through the actual build and renderer

## Trace declared resources to loaded content

Identify the document or entry point, resource references, resolution rules and resulting bytes. Follow dependent resources, such as a stylesheet's font. Resolve relative paths from the referring file within the actual output layout. In the check result, identify input digest, generated address, media type and build evidence for resources supporting the claim.

When local or offline use is required, run the actual renderer in a fresh context with only declared requests permitted. Observe attempted requests and successful loads, and establish that the intended resource loaded. A successful page can still use a fallback. Test a missing local resource, incorrect path base and unapproved external reference. Limit conclusions to tested states; conditional loads elsewhere remain open.

A source file, declared resource, copied output and loaded browser resource establish different facts. Trace fonts, scripts, stylesheets, images and other assets through the build to the URL or import requested. Check dependent path bases after generation, including nested stylesheet URLs and import-map modules. Exercise a fresh build, deep page and deployment prefix when they affect resolution. Verify usability through each intended fallback.

Inspect the font actually drawing the relevant text and glyph. A loaded font can lack the displayed mark or script. Confirm requested image format, dimensions and rendering, and equivalent text for diagrams, mathematics and other visual information. A preflight can flag unsupported profiles, missing resources or disallowed fallbacks; open the output to see what the reader actually receives.

## Worked example

A page's stylesheet loads a local font. Build into fresh output, open with external requests disabled and inspect the request and font used. Move the stylesheet in a test build to break its relative font path. Check that fallback is not counted as the intended font. Repeat for conditional resource loading where applicable.

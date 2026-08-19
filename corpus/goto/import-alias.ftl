<#-- Go-to-definition: imported template.
     Place the cursor on the import path below and trigger go-to-definition to
     jump to `lib/common.ftl`. -->
<#import "lib/common.ftl" as common>

<#-- Once imported, the `common` namespace is a symbol; hover or go-to on it
     resolves to the import statement. -->
common.greet("World")

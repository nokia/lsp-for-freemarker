<#-- Code action: legacy equality operator.
     Using a single '=' where '==' is expected in a comparison produces a
     'legacy_equal_operator' warning with a quick-fix that replaces '=' with
     '=='. -->
<#assign score = 100>
<#if score = 100>Perfect</#if>

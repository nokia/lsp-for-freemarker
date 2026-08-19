<#-- Diagnostic: undefined macro.
     Calling a macro that has no definition (and is not provided by an import)
     triggers the 'undefined_macro' error from the semantic model. -->
<@notDefinedAnywhere />

<#-- Reference: a defined macro, so you can compare with the undefined call
     above. -->
<#macro defined>Hello</#macro>
<@defined />

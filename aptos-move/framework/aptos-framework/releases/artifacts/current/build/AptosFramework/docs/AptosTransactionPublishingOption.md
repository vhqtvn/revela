
<a name="0x1_AptosTransactionPublishingOption"></a>

# Module `0x1::AptosTransactionPublishingOption`



-  [Function `initialize`](#0x1_AptosTransactionPublishingOption_initialize)
-  [Function `set_module_publishing_allowed`](#0x1_AptosTransactionPublishingOption_set_module_publishing_allowed)


<pre><code><b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability">0x1::Capability</a>;
<b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/CoreFramework/docs/DiemTransactionPublishingOption.md#0x1_DiemTransactionPublishingOption">0x1::DiemTransactionPublishingOption</a>;
<b>use</b> <a href="Marker.md#0x1_Marker">0x1::Marker</a>;
</code></pre>



<a name="0x1_AptosTransactionPublishingOption_initialize"></a>

## Function `initialize`



<pre><code><b>public</b> <b>fun</b> <a href="AptosTransactionPublishingOption.md#0x1_AptosTransactionPublishingOption_initialize">initialize</a>(core_resource_account: &signer, script_allow_list: vector&lt;vector&lt;u8&gt;&gt;, module_publishing_allowed: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="AptosTransactionPublishingOption.md#0x1_AptosTransactionPublishingOption_initialize">initialize</a>(
    core_resource_account: &signer,
    script_allow_list: vector&lt;vector&lt;u8&gt;&gt;,
    module_publishing_allowed: bool,
) {
    <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/CoreFramework/docs/DiemTransactionPublishingOption.md#0x1_DiemTransactionPublishingOption_initialize">DiemTransactionPublishingOption::initialize</a>&lt;ChainMarker&gt;(core_resource_account, script_allow_list, module_publishing_allowed);
}
</code></pre>



</details>

<a name="0x1_AptosTransactionPublishingOption_set_module_publishing_allowed"></a>

## Function `set_module_publishing_allowed`



<pre><code><b>public</b> <b>fun</b> <a href="AptosTransactionPublishingOption.md#0x1_AptosTransactionPublishingOption_set_module_publishing_allowed">set_module_publishing_allowed</a>(account: &signer, is_allowed: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="AptosTransactionPublishingOption.md#0x1_AptosTransactionPublishingOption_set_module_publishing_allowed">set_module_publishing_allowed</a>(account: &signer, is_allowed: bool) {
    <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/CoreFramework/docs/DiemTransactionPublishingOption.md#0x1_DiemTransactionPublishingOption_set_module_publishing_allowed">DiemTransactionPublishingOption::set_module_publishing_allowed</a>(is_allowed, <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Capability.md#0x1_Capability_acquire">Capability::acquire</a>(account, &<a href="Marker.md#0x1_Marker_get">Marker::get</a>()));
}
</code></pre>



</details>

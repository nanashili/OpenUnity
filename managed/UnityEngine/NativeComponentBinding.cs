namespace UnityEngine;

internal enum NativeComponentBindingCategory
{
    CreateAndQuery,
    QueryOnly,
}

internal enum NativeComponentBindingOperation
{
    Create,
    Query,
}

[AttributeUsage(AttributeTargets.Class, Inherited = false, AllowMultiple = false)]
internal sealed class NativeComponentBindingAttribute : Attribute
{
    internal NativeComponentBindingAttribute(
        Type adapterType,
        NativeComponentBindingCategory category)
    {
        AdapterType = adapterType;
        Category = category;
    }

    internal Type AdapterType { get; }

    internal NativeComponentBindingCategory Category { get; }
}

internal readonly struct NativeComponentIdentity
{
    internal NativeComponentIdentity(RuntimeContext context, long identity, GameObject owner)
    {
        Context = context;
        Identity = identity;
        Owner = owner;
    }

    internal RuntimeContext Context { get; }

    internal long Identity { get; }

    internal GameObject Owner { get; }
}

internal interface INativeComponentBindingAdapter
{
    Type DeclaredType { get; }

    NativeComponentBindingCategory Category { get; }
}

internal interface INativeComponentQueryBinding : INativeComponentBindingAdapter
{
}

internal interface INativeComponentFactoryBinding : INativeComponentQueryBinding
{
    long Reserve(RuntimeContext context, GameObject owner);

    Component CreateAttached(NativeComponentIdentity identity);

    void Commit(RuntimeContext context, long identity);

    void Abort(RuntimeContext context, long identity);
}

internal sealed class NativeComponentBinding
{
    internal NativeComponentBinding(
        Type declaredType,
        NativeComponentBindingCategory category,
        INativeComponentQueryBinding adapter)
    {
        DeclaredType = declaredType;
        Category = category;
        Adapter = adapter;
        Factory = adapter as INativeComponentFactoryBinding;
    }

    internal Type DeclaredType { get; }

    internal NativeComponentBindingCategory Category { get; }

    internal INativeComponentQueryBinding Adapter { get; }

    internal bool CanCreate => Factory is not null;

    internal INativeComponentFactoryBinding? Factory { get; }
}

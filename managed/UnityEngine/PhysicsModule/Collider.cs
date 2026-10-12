namespace UnityEngine;

[NativeComponentBinding(typeof(ColliderQueryBinding), NativeComponentBindingCategory.QueryOnly)]
public class Collider : Component
{
    public Collider() : this(RejectConstruction())
    {
    }

    private Collider(ConstructionToken token)
        : base(token.Context, token.Identity, token.Owner)
    {
    }

    private static ConstructionToken RejectConstruction() =>
        throw new NotSupportedException("OpenUnity Collider construction is unsupported.");

    private sealed class ConstructionToken
    {
        internal RuntimeContext Context => throw new NotSupportedException();
        internal long Identity => throw new NotSupportedException();
        internal GameObject Owner => throw new NotSupportedException();
    }
}

internal sealed class ColliderQueryBinding : INativeComponentQueryBinding
{
    internal ColliderQueryBinding()
    {
    }

    public Type DeclaredType => typeof(Collider);

    public NativeComponentBindingCategory Category => NativeComponentBindingCategory.QueryOnly;
}

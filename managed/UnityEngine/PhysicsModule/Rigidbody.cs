using OpenUnity.Interop;

namespace UnityEngine;

[NativeComponentBinding(typeof(RigidbodyBinding), NativeComponentBindingCategory.CreateAndQuery)]
public partial class Rigidbody : Component
{
    public Rigidbody() : this(RejectConstruction())
    {
    }

    private Rigidbody(ConstructionToken token)
        : base(token.Context, token.Identity, token.Owner)
    {
    }

    private static ConstructionToken RejectConstruction() =>
        throw new NotSupportedException(
            "OpenUnity Rigidbody construction is supported only through GameObject attachment.");

    private sealed class ConstructionToken
    {
        internal RuntimeContext Context => throw new NotSupportedException();
        internal long Identity => throw new NotSupportedException();
        internal GameObject Owner => throw new NotSupportedException();
    }

    internal Rigidbody(RuntimeContext context, long identity, GameObject owner)
        : base(context, identity, owner) { }

    public Vector3 position
    {
        get => RigidbodyAccess.ReadVector(this, RigidbodyAccess.VectorProperty.Position);
        set => RigidbodyAccess.WriteVector(this, RigidbodyAccess.VectorProperty.Position, value);
    }

    public Vector3 linearVelocity
    {
        get => RigidbodyAccess.ReadVector(this, RigidbodyAccess.VectorProperty.LinearVelocity);
        set => RigidbodyAccess.WriteVector(this, RigidbodyAccess.VectorProperty.LinearVelocity, value);
    }

    public Vector3 angularVelocity
    {
        get => RigidbodyAccess.ReadVector(this, RigidbodyAccess.VectorProperty.AngularVelocity);
        set => RigidbodyAccess.WriteVector(this, RigidbodyAccess.VectorProperty.AngularVelocity, value);
    }
}

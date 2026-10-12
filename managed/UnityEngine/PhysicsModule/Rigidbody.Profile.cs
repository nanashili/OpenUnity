namespace UnityEngine;

public partial class Rigidbody
{
    public float linearDamping
    {
        get => RigidbodyAccess.ReadScalar(this, RigidbodyAccess.ScalarProperty.LinearDamping);
        set => RigidbodyAccess.WriteScalar(this, RigidbodyAccess.ScalarProperty.LinearDamping, value);
    }

    public float angularDamping
    {
        get => RigidbodyAccess.ReadScalar(this, RigidbodyAccess.ScalarProperty.AngularDamping);
        set => RigidbodyAccess.WriteScalar(this, RigidbodyAccess.ScalarProperty.AngularDamping, value);
    }

    public float mass
    {
        get => RigidbodyAccess.ReadScalar(this, RigidbodyAccess.ScalarProperty.Mass);
        set => RigidbodyAccess.WriteScalar(this, RigidbodyAccess.ScalarProperty.Mass, value);
    }

    public bool useGravity
    {
        get => RigidbodyAccess.ReadFlag(this, RigidbodyAccess.FlagProperty.UseGravity);
        set => RigidbodyAccess.WriteFlag(this, RigidbodyAccess.FlagProperty.UseGravity, value);
    }

    public bool isKinematic
    {
        get => RigidbodyAccess.ReadFlag(this, RigidbodyAccess.FlagProperty.IsKinematic);
        set => RigidbodyAccess.WriteFlag(this, RigidbodyAccess.FlagProperty.IsKinematic, value);
    }

    public bool detectCollisions
    {
        get => RigidbodyAccess.ReadFlag(this, RigidbodyAccess.FlagProperty.DetectCollisions);
        set => RigidbodyAccess.WriteFlag(this, RigidbodyAccess.FlagProperty.DetectCollisions, value);
    }

    public RigidbodyInterpolation interpolation
    {
        get => RigidbodyAccess.ReadInterpolation(this);
        set => RigidbodyAccess.WriteInterpolation(this, value);
    }

    public float sleepThreshold
    {
        get => RigidbodyAccess.ReadScalar(this, RigidbodyAccess.ScalarProperty.SleepThreshold);
        set => RigidbodyAccess.WriteScalar(this, RigidbodyAccess.ScalarProperty.SleepThreshold, value);
    }

    public bool IsSleeping() => RigidbodyAccess.ReadFlag(this, RigidbodyAccess.FlagProperty.Sleeping);

    public void WakeUp() => RigidbodyAccess.WakeUp(this);
}

namespace UnityEngine;

public partial class Transform
{
    public Vector3 localEulerAngles
    {
        get => EulerAngles.FromQuaternion(ToNumerics(localRotation));
        set => localRotation = FromNumerics(EulerAngles.ToQuaternion(value));
    }

    public Vector3 eulerAngles
    {
        get => EulerAngles.FromQuaternion(ToNumerics(rotation));
        set => rotation = FromNumerics(EulerAngles.ToQuaternion(value));
    }

    private static Quaternion FromNumerics(System.Numerics.Quaternion value) =>
        new(value.X, value.Y, value.Z, value.W);
}

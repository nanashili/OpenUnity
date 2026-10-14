using OpenUnity.Interop;
using NumericsQuaternion = System.Numerics.Quaternion;
using NumericsVector3 = System.Numerics.Vector3;

namespace UnityEngine;

public partial class Transform : Component
{
    internal Transform(RuntimeContext context, long identity, GameObject owner)
        : base(context, identity, owner) { }

    public Vector3 localPosition { get => ReadVector(1); set => Write(1, [value.x, value.y, value.z]); }
    public Quaternion localRotation { get => ReadQuaternion(2); set => Write(2, [value.x, value.y, value.z, value.w]); }
    public Vector3 localScale { get => ReadVector(3); set => Write(3, [value.x, value.y, value.z]); }
    public Vector3 position { get => ReadVector(4); set => Write(4, [value.x, value.y, value.z]); }
    public Quaternion rotation { get => ReadQuaternion(5); set => Write(5, [value.x, value.y, value.z, value.w]); }

    public Vector3 TransformPoint(Vector3 point) => ConvertPoint(point, false);
    public Vector3 InverseTransformPoint(Vector3 point) => ConvertPoint(point, true);

    public void Translate(Vector3 translation) => Translate(translation, Space.Self);

    public void Translate(Vector3 translation, Space relativeTo)
    {
        CheckSpace(relativeTo);
        Validate([translation.x, translation.y, translation.z]);
        NumericsVector3 displacement = new(translation.x, translation.y, translation.z);
        if (relativeTo == Space.Self)
            displacement = NumericsVector3.Transform(displacement, ToNumerics(rotation));
        Vector3 current = position;
        Write(4, [current.x + displacement.X, current.y + displacement.Y, current.z + displacement.Z]);
    }

    public void Rotate(Vector3 eulers) => Rotate(eulers, Space.Self);

    public void Rotate(Vector3 eulers, Space relativeTo)
    {
        CheckSpace(relativeTo);
        Validate([eulers.x, eulers.y, eulers.z]);
        // Independent Hamilton model from the reviewed contract: Ry * Rx * Rz.
        NumericsQuaternion delta = Axis(eulers.y, 1) * Axis(eulers.x, 0) * Axis(eulers.z, 2);
        NumericsQuaternion result = relativeTo == Space.Self
            ? ToNumerics(localRotation) * delta
            : delta * ToNumerics(rotation);
        Write(relativeTo == Space.Self ? 2 : 5, [result.X, result.Y, result.Z, result.W]);
    }

    private static NumericsQuaternion Axis(float degrees, int axis)
    {
        double half = degrees * (Math.PI / 360.0);
        float sine = (float)Math.Sin(half);
        float cosine = (float)Math.Cos(half);
        return axis switch
        {
            0 => new NumericsQuaternion(sine, 0, 0, cosine),
            1 => new NumericsQuaternion(0, sine, 0, cosine),
            _ => new NumericsQuaternion(0, 0, sine, cosine)
        };
    }

    private static NumericsQuaternion ToNumerics(Quaternion value) => new(value.x, value.y, value.z, value.w);

    private static void CheckSpace(Space relativeTo)
    {
        if (relativeTo != Space.World && relativeTo != Space.Self)
            throw new NotSupportedException($"OpenUnity does not support Space value {(int)relativeTo}.");
    }

    private float[] Read(int property, int count) => Context.Invoke(runtime =>
    {
        float[] values = new float[count];
        CheckNative(OpenUnityNative.ou_transform_read(runtime, Identity, property, values, count), "pose read");
        return values;
    });

    private Vector3 ReadVector(int property)
    {
        float[] values = Read(property, 3);
        return new Vector3(values[0], values[1], values[2]);
    }

    private Quaternion ReadQuaternion(int property)
    {
        float[] values = Read(property, 4);
        return new Quaternion(values[0], values[1], values[2], values[3]);
    }

    private void Write(int property, float[] values)
    {
        Validate(values);
        Context.Invoke(runtime =>
        {
            int status = OpenUnityNative.ou_transform_write(runtime, Identity, property, values, values.Length);
            CheckNative(status, "pose assignment");
            return status;
        });
    }

    private Vector3 ConvertPoint(Vector3 point, bool inverse) => Context.Invoke(runtime =>
    {
        float[] input = [point.x, point.y, point.z];
        float[] output = new float[3];
        CheckNative(OpenUnityNative.ou_transform_point(runtime, Identity, input, output, 3, inverse ? 1 : 0), "point conversion");
        return new Vector3(output[0], output[1], output[2]);
    });

    private static void Validate(float[] values)
    {
        foreach (float value in values)
            if (!float.IsFinite(value))
                throw new ArgumentException("OpenUnity Transform values must be finite.");
    }
}

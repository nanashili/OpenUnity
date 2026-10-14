using NumericsQuaternion = System.Numerics.Quaternion;

namespace UnityEngine;

internal static class EulerAngles
{
    private const double DegreesPerRadian = 180.0 / Math.PI;
    private const double RadiansPerDegree = Math.PI / 180.0;
    private const double GimbalTolerance = 1e-6;

    internal static NumericsQuaternion ToQuaternion(Vector3 degrees)
    {
        double xHalf = degrees.x * RadiansPerDegree / 2.0;
        double yHalf = degrees.y * RadiansPerDegree / 2.0;
        double zHalf = degrees.z * RadiansPerDegree / 2.0;

        NumericsQuaternion aroundX = Axis(xHalf, 0);
        NumericsQuaternion aroundY = Axis(yHalf, 1);
        NumericsQuaternion aroundZ = Axis(zHalf, 2);

        return NumericsQuaternion.Normalize(aroundY * aroundX * aroundZ);
    }

    internal static Vector3 FromQuaternion(NumericsQuaternion value)
    {
        NumericsQuaternion rotation = NumericsQuaternion.Normalize(value);
        double x = rotation.X;
        double y = rotation.Y;
        double z = rotation.Z;
        double w = rotation.W;

        double matrix11 = 1.0 - 2.0 * (y * y + z * z);
        double matrix13 = 2.0 * (x * z + w * y);
        double matrix21 = 2.0 * (x * y + w * z);
        double matrix22 = 1.0 - 2.0 * (x * x + z * z);
        double matrix23 = 2.0 * (y * z - w * x);
        double matrix31 = 2.0 * (x * z - w * y);

        double sineX = Math.Clamp(-matrix23, -1.0, 1.0);
        double angleX = Math.Asin(sineX);
        double cosineX = Math.Sqrt(Math.Max(0.0, 1.0 - sineX * sineX));

        double angleY;
        double angleZ;
        if (cosineX > GimbalTolerance)
        {
            angleY = Math.Atan2(matrix13, 1.0 - 2.0 * (x * x + y * y));
            angleZ = Math.Atan2(matrix21, matrix22);
        }
        else
        {
            angleY = Math.Atan2(-matrix31, matrix11);
            angleZ = 0.0;
        }

        return new Vector3(
            NormalizeDegrees(angleX),
            NormalizeDegrees(angleY),
            NormalizeDegrees(angleZ));
    }

    private static NumericsQuaternion Axis(double halfRadians, int axis)
    {
        float sine = (float)Math.Sin(halfRadians);
        float cosine = (float)Math.Cos(halfRadians);

        return axis switch
        {
            0 => new NumericsQuaternion(sine, 0f, 0f, cosine),
            1 => new NumericsQuaternion(0f, sine, 0f, cosine),
            _ => new NumericsQuaternion(0f, 0f, sine, cosine)
        };
    }

    private static float NormalizeDegrees(double radians)
    {
        double degrees = radians * DegreesPerRadian;
        double normalized = degrees % 360.0;
        if (normalized < 0.0)
            normalized += 360.0;

        if (normalized < 1e-5 || 360.0 - normalized < 1e-5)
            normalized = 0.0;

        return (float)normalized;
    }
}

namespace UnityEngine;

public struct Vector3
{
    public float x;
    public float y;
    public float z;

    public static Vector3 zero => new Vector3(0f, 0f, 0f);

    public Vector3(float x, float y, float z)
    {
        this.x = x;
        this.y = y;
        this.z = z;
    }
}

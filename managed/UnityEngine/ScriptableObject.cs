namespace UnityEngine;

public class ScriptableObject : Object
{
    public ScriptableObject() { }

    public static T CreateInstance<T>() where T : ScriptableObject =>
        (T)Activator.CreateInstance(typeof(T), nonPublic: true)!;
}

using System.Reflection;

namespace UnityEngine;

internal sealed partial class LifecycleDispatcher
{
    internal static void ValidateScriptType(Type type)
    {
        ArgumentNullException.ThrowIfNull(type);
        if (!type.IsClass || type.IsAbstract || type.ContainsGenericParameters ||
            type == typeof(MonoBehaviour) || !typeof(MonoBehaviour).IsAssignableFrom(type) ||
            type.GetConstructor(Type.EmptyTypes) is null)
            throw new NotSupportedException("OpenUnity attachment requires a concrete custom MonoBehaviour with a public parameterless constructor.");
    }

    internal void PreflightSceneType(Type type)
    {
        CheckThread();
        ValidateScriptType(type);
        callbacks[type] = ResolveCallbacks(type);
    }

    internal void PreflightTimedType(Type type)
    {
        CheckThread();
        if (!fixedCallbacks.ContainsKey(type))
            fixedCallbacks.Add(type, ResolveCallback(type, "FixedUpdate"));
    }

    private IReadOnlyDictionary<int, MethodInfo?> ResolveCallbacks(Type type)
    {
        if (callbacks.TryGetValue(type, out IReadOnlyDictionary<int, MethodInfo?>? existing))
            return existing;

        Dictionary<int, MethodInfo?> methods = new();
        foreach ((int eventId, string name) in EventNames)
            methods.Add(eventId, ResolveCallback(type, name));
        return methods;
    }

    private static MethodInfo? ResolveCallback(Type type, string name)
    {
        for (Type? current = type; current != typeof(MonoBehaviour) && current is not null; current = current.BaseType)
        {
            MethodInfo[] candidates = current.GetMethods(BindingFlags.Instance | BindingFlags.Static |
                BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.DeclaredOnly)
                .Where(method => method.Name == name).ToArray();
            if (candidates.Length == 0)
                continue;
            if (candidates.Length != 1 || candidates[0].IsStatic || candidates[0].IsGenericMethod ||
                candidates[0].ReturnType != typeof(void) || candidates[0].GetParameters().Length != 0)
                throw new NotSupportedException(
                    $"OpenUnity lifecycle method {name} must be one instance void parameterless method.");
            return candidates[0];
        }
        return null;
    }
}

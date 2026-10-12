using System.Reflection;
using System.Text.Json;
using UnityEngine;

namespace OpenUnity.Hosting;

internal sealed partial class ScenePlan
{
    private static List<(JsonElement Data, Type Type)> ReadScriptBindings(JsonElement scripts, Assembly assembly,
        LifecycleDispatcher dispatcher, Dictionary<long, Type> identities)
    {
        List<(JsonElement Data, Type Type)> scriptTypes = new();
        Dictionary<string, (string Assembly, string Type, int Order)> bindings = new(StringComparer.Ordinal);
        foreach (JsonElement script in scripts.EnumerateArray())
        {
            Shape(script, "file_id", "owner", "guid", "assembly", "type_name", "enabled", "execution_order", "fields");
            string guid = Text(script.GetProperty("guid"));
            if (guid.Length != 32 || !guid.All(character => character is >= '0' and <= '9' or >= 'a' and <= 'f'))
                throw Invalid("Invalid script GUID.");
            string assemblyName = Text(script.GetProperty("assembly"));
            if (assemblyName != assembly.GetName().Name)
                throw Invalid("Script binding assembly identity does not match the supplied assembly.");
            string typeName = Text(script.GetProperty("type_name"));
            Type type = assembly.GetType(typeName, throwOnError: false, ignoreCase: false)
                ?? throw Invalid("Script binding type was not found.");
            dispatcher.PreflightSceneType(type);
            int order = Integer(script.GetProperty("execution_order"));
            if (ExecutionOrder(type) != order)
                throw Invalid("Script execution order differs from metadata.");
            var binding = (assemblyName, typeName, order);
            if (bindings.TryGetValue(guid, out var previous) && previous != binding)
                throw Invalid("Conflicting script GUID binding.");
            bindings[guid] = binding;
            _ = Id(script.GetProperty("owner"));
            _ = Boolean(script.GetProperty("enabled"));
            AddIdentity(identities, Id(script.GetProperty("file_id")), type);
            scriptTypes.Add((script, type));
        }
        return scriptTypes;
    }

    private static int ExecutionOrder(Type type)
    {
        CustomAttributeData[] attributes = CustomAttributeData.GetCustomAttributes(type)
            .Where(attribute => attribute.AttributeType == typeof(DefaultExecutionOrder)).ToArray();
        if (attributes.Length == 0)
            return 0;
        if (attributes.Length != 1 || attributes[0].ConstructorArguments.Count != 1 ||
            attributes[0].ConstructorArguments[0].ArgumentType != typeof(int) || attributes[0].NamedArguments.Count != 0)
            throw Invalid("Unsupported DefaultExecutionOrder metadata.");
        return (int)attributes[0].ConstructorArguments[0].Value!;
    }

    private static Dictionary<string, FieldInfo> SerializedFields(Type type)
    {
        List<FieldInfo> candidates = new();
        for (Type? current = type; current is not null && current != typeof(MonoBehaviour); current = current.BaseType)
            candidates.AddRange(current.GetFields(BindingFlags.Instance | BindingFlags.Static |
                BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.DeclaredOnly));
        Dictionary<string, FieldInfo> fields = new(StringComparer.Ordinal);
        foreach (FieldInfo field in candidates)
        {
            IList<CustomAttributeData> attributes = CustomAttributeData.GetCustomAttributes(field);
            if (attributes.Any(attribute => attribute.AttributeType == typeof(NonSerializedAttribute)) ||
                (!field.IsPublic && !attributes.Any(attribute => attribute.AttributeType == typeof(SerializeField))))
                continue;
            if (field.IsStatic || field.IsLiteral || field.IsInitOnly ||
                candidates.Count(candidate => candidate.Name == field.Name) != 1)
                throw Invalid("Scene serialized field must be an unambiguous writable instance field.");
            _ = FieldKind(field.FieldType);
            fields.Add(field.Name, field);
        }
        return fields;
    }

    private static string FieldKind(Type type) => type == typeof(int) ? "i32" : type == typeof(float) ? "f32" :
        type == typeof(bool) ? "bool" : type == typeof(string) ? "string" : type == typeof(Vector3) ? "vec3" :
        type == typeof(Quaternion) ? "quat" : type == typeof(GameObject) || type == typeof(Transform) ||
        (type != typeof(MonoBehaviour) && typeof(MonoBehaviour).IsAssignableFrom(type)) ? "reference" :
        throw Invalid("Unsupported serialized field type.");
}

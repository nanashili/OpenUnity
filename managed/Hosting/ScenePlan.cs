using System.Reflection;
using System.Text;
using System.Text.Json;
using UnityEngine;

namespace OpenUnity.Hosting;

// This is the independently specified transfer format, not a Unity scene parser.
internal sealed class ScenePlan
{
    internal const int MaximumBytes = 16 * 1024 * 1024;
    private const int MaximumObjects = 10_000;
    private static readonly UTF8Encoding Utf8 = new(false, true);
    internal sealed record Owner(long Id, long Transform);
    internal sealed record Field(FieldInfo Metadata, object? Value, long? Reference);
    internal sealed record Script(long Id, long Owner, Type Type, Field[] Fields);
    internal Owner[] Owners { get; }
    internal Script[] Scripts { get; }
    private readonly byte[] payload;

    private ScenePlan(byte[] payload, Owner[] owners, Script[] scripts)
    {
        this.payload = payload;
        Owners = owners;
        Scripts = scripts;
    }

    internal long Stage(RuntimeContext context) => context.Invoke(runtime =>
        OpenUnity.Interop.OpenUnityNative.ou_scene_stage(runtime, payload, payload.Length));

    internal static ScenePlan Parse(byte[] payload, Assembly assembly, LifecycleDispatcher dispatcher)
    {
        if (payload.Length is <= 0 or > MaximumBytes)
            throw Invalid("Scene plan byte limit exceeded.");
        if (assembly.GetName().Name != "Assembly-CSharp")
            throw Invalid("Scene scripts require the supplied Assembly-CSharp assembly identity.");
        _ = Utf8.GetString(payload);
        using JsonDocument document = JsonDocument.Parse(payload, new JsonDocumentOptions { MaxDepth = 64 });
        ValidateStringsAndDuplicates(document.RootElement);
        JsonElement root = Shape(document.RootElement, "schema_version", "scene_name", "roots", "game_objects", "transforms", "scripts");
        if (Integer(root.GetProperty("schema_version")) != 1)
            throw Invalid("Unsupported scene plan version.");
        _ = Text(root.GetProperty("scene_name"));
        JsonElement gameObjects = Array(root.GetProperty("game_objects"));
        JsonElement transforms = Array(root.GetProperty("transforms"));
        JsonElement scripts = Array(root.GetProperty("scripts"));
        if ((long)gameObjects.GetArrayLength() + transforms.GetArrayLength() + scripts.GetArrayLength() > MaximumObjects)
            throw Invalid("Scene object limit exceeded.");
        Dictionary<long, Type> identities = new();
        Dictionary<long, long> transformOwners = new();
        foreach (JsonElement transform in transforms.EnumerateArray())
        {
            Shape(transform, "file_id", "owner", "parent", "children", "local_position", "local_rotation", "local_scale");
            long id = Id(transform.GetProperty("file_id"));
            AddIdentity(identities, id, typeof(Transform));
            transformOwners.Add(id, Id(transform.GetProperty("owner")));
            _ = Reference(transform.GetProperty("parent"));
            ReadIds(transform.GetProperty("children"));
            _ = Floats(transform.GetProperty("local_position"), 3);
            float[] rotation = Floats(transform.GetProperty("local_rotation"), 4);
            if (rotation.All(value => value == 0))
                throw Invalid("Transform rotation cannot be zero.");
            _ = Floats(transform.GetProperty("local_scale"), 3);
        }
        List<Owner> owners = new();
        foreach (JsonElement owner in gameObjects.EnumerateArray())
        {
            Shape(owner, "file_id", "name", "active", "components");
            long id = Id(owner.GetProperty("file_id"));
            AddIdentity(identities, id, typeof(GameObject));
            _ = Text(owner.GetProperty("name"));
            _ = Boolean(owner.GetProperty("active"));
            long[] components = ReadIds(owner.GetProperty("components"));
            if (components.Length == 0 || !transformOwners.TryGetValue(components[0], out long actualOwner) || actualOwner != id)
                throw Invalid("GameObject requires its mandatory Transform first.");
            owners.Add(new Owner(id, components[0]));
        }
        ReadIds(root.GetProperty("roots"));
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
        List<Script> prepared = new();
        foreach ((JsonElement data, Type type) in scriptTypes)
        {
            long owner = Id(data.GetProperty("owner"));
            if (!identities.TryGetValue(owner, out Type? ownerType) || ownerType != typeof(GameObject))
                throw Invalid("Script owner is not a scene GameObject.");
            Dictionary<string, FieldInfo> eligible = SerializedFields(type);
            JsonElement values = Array(data.GetProperty("fields"));
            if (values.GetArrayLength() > 256 || values.GetArrayLength() != eligible.Count)
                throw Invalid("Scene fields must exactly match eligible reflected fields.");
            HashSet<string> seen = new(StringComparer.Ordinal);
            List<Field> fields = new();
            foreach (JsonElement value in values.EnumerateArray())
            {
                Shape(value, "name", "value");
                string name = Text(value.GetProperty("name"));
                if (!seen.Add(name) || !eligible.TryGetValue(name, out FieldInfo? field))
                    throw Invalid("Unknown, duplicate or nonserialized scene field.");
                JsonElement tagged = Shape(value.GetProperty("value"), "kind", "value");
                string kind = Text(tagged.GetProperty("kind"));
                if (kind != FieldKind(field.FieldType))
                    throw Invalid("Scene field kind differs from reflected type.");
                if (kind == "reference" && typeof(MonoBehaviour).IsAssignableFrom(field.FieldType) &&
                    !scriptTypes.Any(binding => binding.Type == field.FieldType))
                    throw Invalid("Scene reference field type must have an explicit script binding.");
                JsonElement item = tagged.GetProperty("value");
                object? scalar;
                long? reference = null;
                switch (kind)
                {
                    case "i32": scalar = Integer(item); break;
                    case "f32": scalar = Float(item); break;
                    case "bool": scalar = Boolean(item); break;
                    case "string": scalar = Text(item); break;
                    case "vec3":
                        float[] vector = Floats(item, 3);
                        scalar = new Vector3(vector[0], vector[1], vector[2]); break;
                    case "quat":
                        float[] quaternion = Floats(item, 4);
                        scalar = new Quaternion(quaternion[0], quaternion[1], quaternion[2], quaternion[3]); break;
                    case "reference":
                        reference = Reference(item);
                        if (reference != 0 && (!identities.TryGetValue(reference.Value, out Type? target) || !field.FieldType.IsAssignableFrom(target)))
                            throw Invalid("Scene reference is absent or not assignable.");
                        scalar = null; break;
                    default: throw Invalid("Unsupported scene field kind.");
                }
                fields.Add(new Field(field, scalar, reference));
            }
            prepared.Add(new Script(Id(data.GetProperty("file_id")), owner, type, fields.ToArray()));
        }
        return new ScenePlan(payload, owners.ToArray(), prepared.ToArray());
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
            candidates.AddRange(current.GetFields(BindingFlags.Instance | BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.DeclaredOnly));
        Dictionary<string, FieldInfo> fields = new(StringComparer.Ordinal);
        foreach (FieldInfo field in candidates)
        {
            IList<CustomAttributeData> attributes = CustomAttributeData.GetCustomAttributes(field);
            if (attributes.Any(attribute => attribute.AttributeType == typeof(NonSerializedAttribute)) ||
                (!field.IsPublic && !attributes.Any(attribute => attribute.AttributeType == typeof(SerializeField))))
                continue;
            if (field.IsStatic || field.IsLiteral || field.IsInitOnly || candidates.Count(candidate => candidate.Name == field.Name) != 1)
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

    private static void AddIdentity(Dictionary<long, Type> identities, long id, Type type)
    {
        if (!identities.TryAdd(id, type))
            throw Invalid("Duplicate scene local file ID.");
    }

    private static JsonElement Shape(JsonElement element, params string[] keys)
    {
        if (element.ValueKind != JsonValueKind.Object || element.EnumerateObject().Count() != keys.Length ||
            keys.Any(key => !element.TryGetProperty(key, out _)))
            throw Invalid("Unknown or missing scene plan key.");
        return element;
    }

    private static JsonElement Array(JsonElement element) => element.ValueKind == JsonValueKind.Array
        ? element : throw Invalid("Scene plan array required.");
    private static int Integer(JsonElement element) => element.ValueKind == JsonValueKind.Number && element.TryGetInt32(out int value)
        ? value : throw Invalid("Scene plan Int32 required.");
    private static long Reference(JsonElement element) => element.ValueKind == JsonValueKind.Number && element.TryGetInt64(out long value) && value >= 0
        ? value : throw Invalid("Scene plan nonnegative Int64 reference required.");
    private static long Id(JsonElement element)
    {
        long value = Reference(element);
        return value > 0 ? value : throw Invalid("Scene plan identity must be positive.");
    }
    private static bool Boolean(JsonElement element) => element.ValueKind is JsonValueKind.True or JsonValueKind.False
        ? element.GetBoolean() : throw Invalid("Scene plan Boolean required.");
    private static float Float(JsonElement element) => element.ValueKind == JsonValueKind.Number && element.TryGetSingle(out float value) && float.IsFinite(value)
        ? value : throw Invalid("Scene plan finite Single required.");
    private static float[] Floats(JsonElement element, int count)
    {
        Array(element);
        if (element.GetArrayLength() != count)
            throw Invalid("Scene plan vector length mismatch.");
        return element.EnumerateArray().Select(Float).ToArray();
    }
    private static long[] ReadIds(JsonElement element)
    {
        Array(element);
        if (element.GetArrayLength() > MaximumObjects)
            throw Invalid("Scene identity array limit exceeded.");
        return element.EnumerateArray().Select(Id).ToArray();
    }
    private static string Text(JsonElement element) => element.ValueKind == JsonValueKind.String
        ? element.GetString()! : throw Invalid("Scene plan string required.");

    private static void ValidateStringsAndDuplicates(JsonElement element)
    {
        if (element.ValueKind == JsonValueKind.Object)
        {
            HashSet<string> keys = new(StringComparer.Ordinal);
            foreach (JsonProperty property in element.EnumerateObject())
            {
                if (Utf8.GetByteCount(property.Name) > 1_048_576 || !keys.Add(property.Name))
                    throw Invalid("Oversized or duplicate scene plan key.");
                ValidateStringsAndDuplicates(property.Value);
            }
        }
        else if (element.ValueKind == JsonValueKind.Array)
            foreach (JsonElement item in element.EnumerateArray())
                ValidateStringsAndDuplicates(item);
        else if (element.ValueKind == JsonValueKind.String && Utf8.GetByteCount(Text(element)) > 1_048_576)
            throw Invalid("Oversized scene plan string.");
    }

    private static ArgumentException Invalid(string message) => new(message);
}

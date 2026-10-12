using System.Reflection;
using System.Text.Json;
using UnityEngine;

namespace OpenUnity.Hosting;

internal sealed partial class ScenePlan
{
    private static List<Script> ReadScripts(List<(JsonElement Data, Type Type)> scriptTypes,
        Dictionary<long, Type> identities)
    {
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
            List<Field> fields = ReadFields(values, eligible, scriptTypes, identities);
            prepared.Add(new Script(Id(data.GetProperty("file_id")), owner, type, fields.ToArray()));
        }
        return prepared;
    }

    private static List<Field> ReadFields(JsonElement values, Dictionary<string, FieldInfo> eligible,
        List<(JsonElement Data, Type Type)> scriptTypes, Dictionary<long, Type> identities)
    {
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
            fields.Add(ReadFieldValue(item, kind, field, identities));
        }
        return fields;
    }

    private static Field ReadFieldValue(JsonElement item, string kind, FieldInfo field,
        Dictionary<long, Type> identities)
    {
        object? scalar;
        long? reference = null;
        switch (kind)
        {
            case "i32":
                scalar = Integer(item);
                break;
            case "f32":
                scalar = Float(item);
                break;
            case "bool":
                scalar = Boolean(item);
                break;
            case "string":
                scalar = Text(item);
                break;
            case "vec3":
                float[] vector = Floats(item, 3);
                scalar = new Vector3(vector[0], vector[1], vector[2]);
                break;
            case "quat":
                float[] quaternion = Floats(item, 4);
                scalar = new Quaternion(quaternion[0], quaternion[1], quaternion[2], quaternion[3]);
                break;
            case "reference":
                reference = Reference(item);
                if (reference != 0 && (!identities.TryGetValue(reference.Value, out Type? target) ||
                    !field.FieldType.IsAssignableFrom(target)))
                    throw Invalid("Scene reference is absent or not assignable.");
                scalar = null;
                break;
            default:
                throw Invalid("Unsupported scene field kind.");
        }
        return new Field(field, scalar, reference);
    }
}

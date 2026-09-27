/** Encodes requested native property values for the standard catalog serializers. */
class TBD_EquipmentNativeJson
{
	//! Preserves native scalar and ordered-array values; unsupported reads fail explicitly.
	static bool Read(BaseContainer source, string property, string nativeType, out string result)
	{
		JsonSaveContainer writer = new JsonSaveContainer();
		ContainerSerializationSaveContext context = new ContainerSerializationSaveContext(false);
		context.SetContainer(writer);
		bool ok;
		if (nativeType == "SCALAR")
		{
			float scalar;
			ok = source.Get(property, scalar);
			if (ok) context.WriteValue("v", scalar);
		}
		else if (nativeType == "INTEGER" || nativeType == "ENUM" || nativeType == "FLAGS")
		{
			int integer;
			ok = source.Get(property, integer);
			if (ok) context.WriteValue("v", integer);
		}
		else if (nativeType == "BOOLEAN")
		{
			bool flag;
			ok = source.Get(property, flag);
			if (ok) context.WriteValue("v", flag);
		}
		else if (nativeType == "STRING" || nativeType == "RESOURCE_NAME" || nativeType == "TYPENAME")
		{
			string text;
			ok = source.Get(property, text);
			if (ok) context.WriteValue("v", text);
		}
		else if (nativeType == "VECTOR3" || nativeType == "VECTOR2")
		{
			vector tuple;
			ok = source.Get(property, tuple);
			array<float> coordinates = {tuple[0], tuple[1]};
			if (nativeType == "VECTOR3") coordinates.Insert(tuple[2]);
			if (ok) context.WriteValue("v", coordinates);
		}
		else if (nativeType == "COLOR")
		{
			Color color = new Color();
			ok = source.Get(property, color);
			if (ok)
			{
				array<float> rgba = {color.R(), color.G(), color.B(), color.A()};
				context.WriteValue("v", rgba);
			}
		}
		else if (nativeType == "VECTOR2_ARRAY" || nativeType == "VECTOR3_ARRAY")
		{
			array<vector> tuples = {};
			ok = source.Get(property, tuples);
			array<ref array<float>> numbers = {};
			foreach (vector point : tuples)
			{
				array<float> coordinates = {point[0], point[1]};
				if (nativeType == "VECTOR3_ARRAY") coordinates.Insert(point[2]);
				numbers.Insert(coordinates);
			}
			if (ok) context.WriteValue("v", numbers);
		}
		else if (nativeType == "SCALAR_ARRAY")
		{
			array<float> scalars = {};
			ok = source.Get(property, scalars);
			if (ok) context.WriteValue("v", scalars);
		}
		else if (nativeType == "INTEGER_ARRAY" || nativeType == "ENUM_ARRAY")
		{
			array<int> integers = {};
			ok = source.Get(property, integers);
			if (ok) context.WriteValue("v", integers);
		}
		else if (nativeType == "BOOLEAN_ARRAY")
		{
			array<bool> flags = {};
			ok = source.Get(property, flags);
			if (ok) context.WriteValue("v", flags);
		}
		else if (nativeType == "STRING_ARRAY" || nativeType == "RESOURCE_NAME_ARRAY")
		{
			array<string> strings = {};
			ok = source.Get(property, strings);
			if (ok) context.WriteValue("v", strings);
		}
		if (!ok) { result = "null"; return false; }
		string encoded = writer.SaveToString();
		result = TBD_EquipmentExportJson.UnwrapNativeValue(encoded);
		return true;
	}
}

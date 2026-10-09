class TBD_SourcePropertyReader
{
	static bool Read(BaseContainer container, string property, DataVarType nativeType, out string value)
	{
		int integer;
		float scalar;
		bool boolean;
		string text;
		vector tuple;
		switch (nativeType)
		{
			case DataVarType.INTEGER:
			case DataVarType.FLAGS:
				if (!container.Get(property, integer)) return false;
				value = integer.ToString(); return true;
			case DataVarType.SCALAR:
				if (!container.Get(property, scalar)) return false;
				value = TBD_SourceExportJson.Scalar(scalar); return true;
			case DataVarType.BOOLEAN:
				if (!container.Get(property, boolean)) return false;
				value = "false";
				if (boolean) value = "true";
				return true;
			case DataVarType.STRING:
			case DataVarType.RESOURCE_NAME:
				if (!container.Get(property, text)) return false;
				value = TBD_SourceExportJson.Quote(text); return true;
			case DataVarType.VECTOR2:
				if (!container.Get(property, tuple)) return false;
				array<float> coordinates = {tuple[0], tuple[1]};
				value = TBD_SourceExportJson.Scalars(coordinates); return true;
			case DataVarType.VECTOR3:
				if (!container.Get(property, tuple)) return false;
				value = TBD_SourceExportJson.VectorValue(tuple); return true;
			case DataVarType.COLOR:
				Color color;
				if (!container.Get(property, color) || !color) return false;
				array<float> rgba = {color.R(), color.G(), color.B(), color.A()};
				value = TBD_SourceExportJson.Scalars(rgba); return true;
			case DataVarType.VECTOR2_ARRAY:
				array<vector> pairs = {};
				if (!container.Get(property, pairs)) return false;
				array<string> pairJson = {};
				foreach (vector pair : pairs)
				{
					array<float> xy = {pair[0], pair[1]};
					pairJson.Insert(TBD_SourceExportJson.Scalars(xy));
				}
				value = "[" + TBD_SourceExportJson.Join(pairJson) + "]"; return true;
			case DataVarType.SCALAR_ARRAY:
				array<float> scalars = {};
				if (!container.Get(property, scalars)) return false;
				value = TBD_SourceExportJson.Scalars(scalars); return true;
			case DataVarType.INTEGER_ARRAY:
				array<int> integers = {};
				if (!container.Get(property, integers)) return false;
				value = TBD_SourceExportJson.Integers(integers); return true;
			case DataVarType.BOOLEAN_ARRAY:
				array<bool> booleans = {};
				if (!container.Get(property, booleans)) return false;
				value = TBD_SourceExportJson.Booleans(booleans); return true;
			case DataVarType.STRING_ARRAY:
			case DataVarType.RESOURCE_NAME_ARRAY:
				array<string> texts = {};
				if (!container.Get(property, texts)) return false;
				value = TBD_SourceExportJson.Strings(texts); return true;
			case DataVarType.VECTOR3_ARRAY:
				array<vector> tuples = {};
				if (!container.Get(property, tuples)) return false;
				array<string> encoded = {};
				foreach (vector item : tuples) encoded.Insert(TBD_SourceExportJson.VectorValue(item));
				value = "[" + TBD_SourceExportJson.Join(encoded) + "]"; return true;
		}
		return false;
	}
}

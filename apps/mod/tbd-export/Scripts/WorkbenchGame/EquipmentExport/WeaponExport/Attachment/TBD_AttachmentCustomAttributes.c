//------------------------------------------------------------------------------------------------
// TBD_AttachmentCustomAttributes.c
//
// Reads the custom attribute list off an Attributes container for this domain.
//
// Core's TBD_EquipmentDisplayAttributes reads a wrapped list under the key CustomAttributes; this
// domain reads it under m_aAttributes and falls back to the flat list when the wrapped one is
// empty. The two keys are why this domain does not share Core's reader.
//------------------------------------------------------------------------------------------------

class TBD_AttachmentCustomAttributes
{
	//------------------------------------------------------------------------------------------------
	//! Retrieve CustomAttributes list from SCR_ItemAttributeCollection or BaseContainer.
	//! Handles both direct BaseContainerList on Attributes and wrapped container layouts.
	static BaseContainerList GetCustomAttributes(BaseContainer attrs)
	{
		if (!attrs)
			return null;

		BaseContainerList list = attrs.GetObjectArray("CustomAttributes");
		if (list && list.Count() > 0)
			return list;

		BaseContainer customContainer = attrs.GetObject("CustomAttributes");
		if (customContainer)
		{
			BaseContainerList wrapped = customContainer.GetObjectArray("m_aAttributes");
			if (wrapped && wrapped.Count() > 0)
				return wrapped;
		}

		if (list)
			return list;

		return null;
	}
}

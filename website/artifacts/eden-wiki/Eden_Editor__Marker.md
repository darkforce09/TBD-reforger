# Eden_Editor:_Marker

Source: https://community.bohemia.net/wiki/Eden_Editor:_Marker

---

A marker is an image shown on the player's map. It is visible to every player, and it is visible only on the map, not in the scene.

### Icon Markers

A marker can be an icon. It is a single image which has a constant size on the screen when zooming the map in and out. When you set its text, it will be shown on its right side. It is used to mark points of interest, like an enemy base or an insertion point.

### Area Markers

Markers can also be areas. Their size is set in meters and is constant in the world space. They are used for marking specific zones, such as an enemy area or border line.
Similarly to trigger areas, the marker area can also be edited using the area scaling widget. 

The area is always shown, but by default you cannot interact with it. However, when the marker is selected, you can drag it by its area as well. 

Markers are a powerful tool for explaining your intentions to the player with little or no text. Do not forget to use them while designing your scenario.

### Attributes

Info

Development

Name

Category

Description

Property

Type

Correspondence

| Type

| Type

| Icon texture.

| itemClass

| String

| markerType

| Variable Name

| Init

| Unique system name. Can contain any characters. The name is not case sensitive, so 'someName' and 'SOMENAME' are treated as the same variables.

| markerName

| String

| createMarker

| Text

| Init

| Text displayed right from the marker.

| text

| String

| markerText

| Position

| Transformation

| World coordinates in meters. X goes from West to East and Y from South to North.

| position

| Position3D

| markerPos

| Size

| Transformation

| Marker A and B size.

| size2

| Array

| markerSize

| Rotation

| Transformation

| Rotation in degrees.

| rotation

| Number

| markerDir

| Shape

| Style

| Marker shape. The marker has to be created as shape marker, for this use "" empty string as class name with create3DENEntity, this will create rectangular marker.
Available options:

- 0 - rectangle

- 1 - ellipse

| markerType

| Number

| markerShape

| Brush

| Style

| Area fill texture.

| brush

| String

| markerBrush

| Color

| Style

| Marker color. 'Default' is based on the selected marker type. Since  2.22 custom color format is supported as well. See setMarkerColor.

| baseColor

| Array

| markerColor

| Alpha

| Style

| Transparency. When the icon marker has a shadow, it will be visible behind the transparent icon.

| alpha

| Number

| markerAlpha

Retrieved from "https://community.bistudio.com/wiki?title=Eden_Editor:_Marker&oldid=376659"
					Category: - Eden Editor: Asset Types
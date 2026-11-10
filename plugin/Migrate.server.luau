-- Copyright (C) 2026 3vil17. AGPL-3.0 with additional terms, see LICENSE and NOTICE.md.
local HttpService = game:GetService("HttpService")
local ChangeHistoryService = game:GetService("ChangeHistoryService")
local ScriptEditorService = game:GetService("ScriptEditorService")
local AssetService = game:GetService("AssetService")
local RunService = game:GetService("RunService")
local TweenService = game:GetService("TweenService")
local Selection = game:GetService("Selection")
local SerializationService = game:GetService("SerializationService")

if not RunService:IsEdit() then
	return
end

local pluginVersion = "dev"
local serverUrl = "http://127.0.0.1:48620"
local regular = Font.new("rbxasset://fonts/families/Montserrat.json", Enum.FontWeight.Regular)
local medium = Font.new("rbxasset://fonts/families/Montserrat.json", Enum.FontWeight.Medium)
local semibold = Font.new("rbxasset://fonts/families/Montserrat.json", Enum.FontWeight.SemiBold)
local bold = Font.new("rbxasset://fonts/families/Montserrat.json", Enum.FontWeight.Bold)
local mono = Font.new("rbxasset://fonts/families/RobotoMono.json", Enum.FontWeight.Medium)
local quick = TweenInfo.new(0.15, Enum.EasingStyle.Quad, Enum.EasingDirection.Out)
local smooth = TweenInfo.new(0.35, Enum.EasingStyle.Quad, Enum.EasingDirection.Out)

local palettes = {
	dark = {
		page = Color3.fromRGB(22, 22, 22),
		card = Color3.fromRGB(28, 30, 32),
		field = Color3.fromRGB(22, 22, 22),
		button = Color3.fromRGB(39, 41, 43),
		buttonHover = Color3.fromRGB(51, 53, 55),
		main = Color3.fromRGB(55, 57, 59),
		mainHover = Color3.fromRGB(69, 71, 73),
		line = Color3.fromRGB(255, 255, 255),
		track = Color3.fromRGB(44, 46, 48),
		scroll = Color3.fromRGB(90, 90, 90),
		text = Color3.fromRGB(212, 212, 212),
		bright = Color3.fromRGB(255, 255, 255),
		label = Color3.fromRGB(170, 170, 170),
		muted = Color3.fromRGB(136, 136, 136),
		dim = Color3.fromRGB(100, 100, 100),
		placeholder = Color3.fromRGB(64, 64, 64),
		green = Color3.fromRGB(59, 234, 87),
		okText = Color3.fromRGB(59, 186, 87),
		red = Color3.fromRGB(236, 59, 71),
		errText = Color3.fromRGB(208, 96, 112),
		accent = "#3BEA57",
	},
	light = {
		page = Color3.fromRGB(243, 243, 243),
		card = Color3.fromRGB(255, 255, 255),
		field = Color3.fromRGB(246, 246, 246),
		button = Color3.fromRGB(241, 241, 241),
		buttonHover = Color3.fromRGB(229, 229, 229),
		main = Color3.fromRGB(226, 226, 226),
		mainHover = Color3.fromRGB(212, 212, 212),
		line = Color3.fromRGB(0, 0, 0),
		track = Color3.fromRGB(226, 226, 226),
		scroll = Color3.fromRGB(180, 180, 180),
		text = Color3.fromRGB(40, 40, 40),
		bright = Color3.fromRGB(15, 15, 15),
		label = Color3.fromRGB(90, 90, 90),
		muted = Color3.fromRGB(105, 105, 105),
		dim = Color3.fromRGB(135, 135, 135),
		placeholder = Color3.fromRGB(190, 190, 190),
		green = Color3.fromRGB(46, 196, 72),
		okText = Color3.fromRGB(28, 140, 52),
		red = Color3.fromRGB(220, 50, 62),
		errText = Color3.fromRGB(190, 40, 55),
		accent = "#1C8C34",
	},
}
local toned = { "BackgroundColor3", "TextColor3", "ImageColor3", "Color", "PlaceholderColor3", "ScrollBarImageColor3" }
local palette = palettes.dark

local icons = {
	plug = { "rbxassetid://136111683851629", 525, 800 },
	unplug = { "rbxassetid://135883070486325", 125, 275 },
	scan = { "rbxassetid://136111683851629", 550, 900 },
	refresh = { "rbxassetid://136111683851629", 400, 975 },
	check = { "rbxassetid://136111683851629", 625, 50 },
	alert = { "rbxassetid://136111683851629", 175, 475 },
	move = { "rbxassetid://136111683851629", 275, 25 },
	link = { "rbxassetid://136111683851629", 250, 825 },
	radio = { "rbxassetid://136111683851629", 450, 900 },
	loader = { "rbxassetid://136111683851629", 525, 575 },
}

local watched = {
	Sound = { "SoundId" },
	MeshPart = { "MeshId", "TextureID" },
	SpecialMesh = { "MeshId", "TextureId" },
	FileMesh = { "MeshId", "TextureId" },
	Decal = { "Texture" },
	Texture = { "Texture" },
	ImageLabel = { "Image", "HoverImage", "PressedImage" },
	ImageButton = { "Image", "HoverImage", "PressedImage" },
	ParticleEmitter = { "Texture" },
	Beam = { "Texture" },
	Trail = { "Texture" },
	Animation = { "AnimationId" },
	Shirt = { "ShirtTemplate" },
	Pants = { "PantsTemplate" },
	ShirtGraphic = { "Graphic" },
	Tool = { "TextureId" },
	Sky = { "SkyboxBk", "SkyboxDn", "SkyboxFt", "SkyboxLf", "SkyboxRt", "SkyboxUp", "SunTextureId", "MoonTextureId" },
}

local token = plugin:GetSetting("rbxyoinkerToken") or ""
local refs = {}
local scriptRefs = {}
local lastMap = {}
local seenRun = -1
local appVersion = nil
local busy = false
local noteMessage = nil
local noteKind = nil
local scope = "place"
local leftovers = {}

local function tone(inst, map)
	for prop, key in map do
		inst:SetAttribute("Tone" .. prop, key)
		inst[prop] = palette[key]
	end
end

local function setIcon(image, icon)
	image.Image = icon[1]
	image.ImageRectOffset = Vector2.new(icon[2], icon[3])
end

local toolbar = plugin:CreateToolbar("rbxyoinker")
local toggle = toolbar:CreateButton("Migrate", "Move this place's assets to another group with rbxyoinker", "rbxassetid://87991195982058")
toggle.ClickableWhenViewportHidden = true

local widget = plugin:CreateDockWidgetPluginGui(
	"RbxyoinkerMigratePanel",
	DockWidgetPluginGuiInfo.new(Enum.InitialDockState.Right, false, false, 340, 600, 240, 320)
)
widget.Title = "rbxyoinker"

local root = Instance.new("ScrollingFrame")
root.Size = UDim2.fromScale(1, 1)
root.BorderSizePixel = 0
root.CanvasSize = UDim2.new()
root.AutomaticCanvasSize = Enum.AutomaticSize.Y
root.ScrollingDirection = Enum.ScrollingDirection.Y
root.VerticalScrollBarInset = Enum.ScrollBarInset.ScrollBar
root.ScrollBarThickness = 5
tone(root, { BackgroundColor3 = "page", ScrollBarImageColor3 = "scroll" })
root.Parent = widget

local rootPadding = Instance.new("UIPadding")
rootPadding.PaddingTop = UDim.new(0, 14)
rootPadding.PaddingBottom = UDim.new(0, 14)
rootPadding.PaddingLeft = UDim.new(0, 12)
rootPadding.PaddingRight = UDim.new(0, 12)
rootPadding.Parent = root

local rootLayout = Instance.new("UIListLayout")
rootLayout.Padding = UDim.new(0, 10)
rootLayout.SortOrder = Enum.SortOrder.LayoutOrder
rootLayout.Parent = root

local header = Instance.new("Frame")
header.BackgroundTransparency = 1
header.Size = UDim2.new(1, 0, 0, 30)
header.LayoutOrder = 1
header.Parent = root

local title = Instance.new("TextLabel")
title.BackgroundTransparency = 1
title.Size = UDim2.fromOffset(0, 30)
title.AutomaticSize = Enum.AutomaticSize.X
title.FontFace = bold
title.RichText = true
title.TextSize = 20
title.TextXAlignment = Enum.TextXAlignment.Left
tone(title, { TextColor3 = "bright" })
title.Parent = header

local round = Instance.new("UICorner")
round.CornerRadius = UDim.new(1, 0)
local hairline = Instance.new("UIStroke")
hairline.ApplyStrokeMode = Enum.ApplyStrokeMode.Border
hairline.Transparency = 0.9
tone(hairline, { Color = "line" })

local pill = Instance.new("Frame")
pill.AnchorPoint = Vector2.new(1, 0.5)
pill.Position = UDim2.fromScale(1, 0.5)
pill.Size = UDim2.fromOffset(0, 26)
pill.AutomaticSize = Enum.AutomaticSize.X
tone(pill, { BackgroundColor3 = "button" })
pill.Parent = header
round:Clone().Parent = pill
hairline:Clone().Parent = pill
local pillPadding = Instance.new("UIPadding")
pillPadding.PaddingLeft = UDim.new(0, 10)
pillPadding.PaddingRight = UDim.new(0, 11)
pillPadding.Parent = pill
local pillLayout = Instance.new("UIListLayout")
pillLayout.FillDirection = Enum.FillDirection.Horizontal
pillLayout.VerticalAlignment = Enum.VerticalAlignment.Center
pillLayout.Padding = UDim.new(0, 7)
pillLayout.SortOrder = Enum.SortOrder.LayoutOrder
pillLayout.Parent = pill

local dotHolder = Instance.new("Frame")
dotHolder.BackgroundTransparency = 1
dotHolder.Size = UDim2.fromOffset(8, 8)
dotHolder.LayoutOrder = 1
dotHolder.Parent = pill

local ring = Instance.new("Frame")
ring.AnchorPoint = Vector2.new(0.5, 0.5)
ring.Position = UDim2.fromScale(0.5, 0.5)
ring.Size = UDim2.fromOffset(8, 8)
ring.BackgroundTransparency = 1
ring.BorderSizePixel = 0
tone(ring, { BackgroundColor3 = "green" })
ring.Parent = dotHolder
round:Clone().Parent = ring

local dot = Instance.new("Frame")
dot.AnchorPoint = Vector2.new(0.5, 0.5)
dot.Position = UDim2.fromScale(0.5, 0.5)
dot.Size = UDim2.fromOffset(8, 8)
dot.BorderSizePixel = 0
tone(dot, { BackgroundColor3 = "dim" })
dot.Parent = dotHolder
round:Clone().Parent = dot

local pillText = Instance.new("TextLabel")
pillText.BackgroundTransparency = 1
pillText.Size = UDim2.fromOffset(0, 26)
pillText.AutomaticSize = Enum.AutomaticSize.X
pillText.FontFace = medium
pillText.TextSize = 12
pillText.LayoutOrder = 2
tone(pillText, { TextColor3 = "text" })
pillText.Parent = pill

local cardTemplate = Instance.new("Frame")
cardTemplate.Size = UDim2.fromScale(1, 0)
cardTemplate.AutomaticSize = Enum.AutomaticSize.Y
tone(cardTemplate, { BackgroundColor3 = "card" })
local cardCorner = Instance.new("UICorner")
cardCorner.CornerRadius = UDim.new(0, 8)
cardCorner.Parent = cardTemplate
local cardStroke = hairline:Clone()
cardStroke.Transparency = 0
cardStroke.Parent = cardTemplate
local cardGradient = Instance.new("UIGradient")
cardGradient.Rotation = 90
cardGradient.Transparency = NumberSequence.new(0.88, 0.96)
cardGradient.Parent = cardStroke
local cardPadding = Instance.new("UIPadding")
cardPadding.PaddingTop = UDim.new(0, 14)
cardPadding.PaddingBottom = UDim.new(0, 14)
cardPadding.PaddingLeft = UDim.new(0, 14)
cardPadding.PaddingRight = UDim.new(0, 14)
cardPadding.Parent = cardTemplate
local cardLayout = Instance.new("UIListLayout")
cardLayout.Padding = UDim.new(0, 10)
cardLayout.SortOrder = Enum.SortOrder.LayoutOrder
cardLayout.Parent = cardTemplate

local headingTemplate = Instance.new("Frame")
headingTemplate.BackgroundTransparency = 1
headingTemplate.Size = UDim2.new(1, 0, 0, 18)
headingTemplate.LayoutOrder = 1
local iconTemplate = Instance.new("ImageLabel")
iconTemplate.Name = "Icon"
iconTemplate.BackgroundTransparency = 1
iconTemplate.Size = UDim2.fromOffset(15, 15)
iconTemplate.ImageRectSize = Vector2.new(24, 24)
iconTemplate.LayoutOrder = 1
tone(iconTemplate, { ImageColor3 = "label" })
local headingIcon = iconTemplate:Clone()
headingIcon.AnchorPoint = Vector2.new(0, 0.5)
headingIcon.Position = UDim2.fromScale(0, 0.5)
headingIcon.Parent = headingTemplate
local headingText = Instance.new("TextLabel")
headingText.Name = "Label"
headingText.BackgroundTransparency = 1
headingText.Position = UDim2.fromOffset(22, 0)
headingText.Size = UDim2.new(1, -22, 1, 0)
headingText.FontFace = semibold
headingText.TextSize = 14
headingText.TextTruncate = Enum.TextTruncate.AtEnd
headingText.TextXAlignment = Enum.TextXAlignment.Left
tone(headingText, { TextColor3 = "label" })
headingText.Parent = headingTemplate
local headingAside = Instance.new("TextLabel")
headingAside.Name = "Aside"
headingAside.BackgroundTransparency = 1
headingAside.AnchorPoint = Vector2.new(1, 0)
headingAside.Position = UDim2.fromScale(1, 0)
headingAside.Size = UDim2.fromScale(0.4, 1)
headingAside.FontFace = medium
headingAside.Text = ""
headingAside.TextSize = 12
headingAside.TextTruncate = Enum.TextTruncate.AtEnd
headingAside.TextXAlignment = Enum.TextXAlignment.Right
tone(headingAside, { TextColor3 = "dim" })
headingAside.Parent = headingTemplate

local hintTemplate = Instance.new("TextLabel")
hintTemplate.BackgroundTransparency = 1
hintTemplate.Size = UDim2.fromScale(1, 0)
hintTemplate.AutomaticSize = Enum.AutomaticSize.Y
hintTemplate.FontFace = regular
hintTemplate.LineHeight = 1.15
hintTemplate.TextSize = 13
hintTemplate.TextWrapped = true
hintTemplate.TextXAlignment = Enum.TextXAlignment.Left
hintTemplate.LayoutOrder = 2
tone(hintTemplate, { TextColor3 = "muted" })

local buttonCorner = Instance.new("UICorner")
buttonCorner.CornerRadius = UDim.new(0, 6)

local buttonTemplate = Instance.new("TextButton")
buttonTemplate.Size = UDim2.new(1, 0, 0, 34)
buttonTemplate.AutoButtonColor = false
buttonTemplate.Text = ""
tone(buttonTemplate, { BackgroundColor3 = "button" })
buttonCorner:Clone().Parent = buttonTemplate
hairline:Clone().Parent = buttonTemplate
local buttonScale = Instance.new("UIScale")
buttonScale.Parent = buttonTemplate
local buttonLayout = Instance.new("UIListLayout")
buttonLayout.FillDirection = Enum.FillDirection.Horizontal
buttonLayout.HorizontalAlignment = Enum.HorizontalAlignment.Center
buttonLayout.VerticalAlignment = Enum.VerticalAlignment.Center
buttonLayout.Padding = UDim.new(0, 7)
buttonLayout.SortOrder = Enum.SortOrder.LayoutOrder
buttonLayout.Parent = buttonTemplate
local buttonIcon = iconTemplate:Clone()
buttonIcon.Size = UDim2.fromOffset(14, 14)
tone(buttonIcon, { ImageColor3 = "text" })
buttonIcon.Parent = buttonTemplate
local buttonText = Instance.new("TextLabel")
buttonText.Name = "Label"
buttonText.BackgroundTransparency = 1
buttonText.Size = UDim2.fromOffset(0, 20)
buttonText.AutomaticSize = Enum.AutomaticSize.X
buttonText.FontFace = medium
buttonText.TextSize = 13
buttonText.LayoutOrder = 2
tone(buttonText, { TextColor3 = "text" })
buttonText.Parent = buttonTemplate

local mainTemplate = buttonTemplate:Clone()
tone(mainTemplate, { BackgroundColor3 = "main" })
tone(mainTemplate.Label, { TextColor3 = "bright" })
tone(mainTemplate.Icon, { ImageColor3 = "bright" })
mainTemplate.UIStroke.Transparency = 0.82

local function makeHeading(card, icon, label)
	local heading = headingTemplate:Clone()
	setIcon(heading.Icon, icon)
	heading.Label.Text = label
	heading.Parent = card
	return heading
end

local connectCard = cardTemplate:Clone()
connectCard.LayoutOrder = 2
connectCard.Parent = root
makeHeading(connectCard, icons.plug, "Connection")

local pairBox = Instance.new("Frame")
pairBox.BackgroundTransparency = 1
pairBox.Size = UDim2.fromScale(1, 0)
pairBox.AutomaticSize = Enum.AutomaticSize.Y
pairBox.LayoutOrder = 2
pairBox.Parent = connectCard
cardLayout:Clone().Parent = pairBox

local connectHint = hintTemplate:Clone()
connectHint.Text = "In the app, open the Migrate tab and click Get a code, then type it in here. You only do this once."
connectHint.LayoutOrder = 1
connectHint.Parent = pairBox

local codeBox = Instance.new("TextBox")
codeBox.Size = UDim2.new(1, 0, 0, 50)
codeBox.ClearTextOnFocus = false
codeBox.FontFace = mono
codeBox.PlaceholderText = "000000"
codeBox.Text = ""
codeBox.TextSize = 28
codeBox.LayoutOrder = 2
tone(codeBox, { BackgroundColor3 = "field", TextColor3 = "bright", PlaceholderColor3 = "placeholder" })
codeBox.Parent = pairBox
buttonCorner:Clone().Parent = codeBox
local codeStroke = hairline:Clone()
codeStroke.Transparency = 0.91
codeStroke.Parent = codeBox

local connectButton = mainTemplate:Clone()
setIcon(connectButton.Icon, icons.link)
connectButton.Label.Text = "Connect"
connectButton.LayoutOrder = 3
connectButton.Parent = pairBox

local pairedRow = Instance.new("Frame")
pairedRow.BackgroundTransparency = 1
pairedRow.Size = UDim2.new(1, 0, 0, 28)
pairedRow.LayoutOrder = 3
pairedRow.Parent = connectCard

local pairedText = hintTemplate:Clone()
pairedText.AutomaticSize = Enum.AutomaticSize.None
pairedText.Size = UDim2.new(1, -110, 1, 0)
pairedText.TextWrapped = false
pairedText.TextTruncate = Enum.TextTruncate.AtEnd
pairedText.Text = "This Studio is paired with the app."
pairedText.Parent = pairedRow

local disconnectButton = buttonTemplate:Clone()
disconnectButton.AnchorPoint = Vector2.new(1, 0.5)
disconnectButton.Position = UDim2.fromScale(1, 0.5)
disconnectButton.Size = UDim2.fromOffset(0, 28)
disconnectButton.AutomaticSize = Enum.AutomaticSize.X
setIcon(disconnectButton.Icon, icons.unplug)
disconnectButton.Icon.Size = UDim2.fromOffset(13, 13)
disconnectButton.Label.Text = "Disconnect"
disconnectButton.Label.TextSize = 12
tone(disconnectButton.Label, { TextColor3 = "muted" })
tone(disconnectButton.Icon, { ImageColor3 = "muted" })
disconnectButton.Parent = pairedRow
local disconnectPadding = Instance.new("UIPadding")
disconnectPadding.PaddingLeft = UDim.new(0, 10)
disconnectPadding.PaddingRight = UDim.new(0, 10)
disconnectPadding.Parent = disconnectButton

local scanCard = cardTemplate:Clone()
scanCard.LayoutOrder = 3
scanCard.Parent = root
local scanHeading = makeHeading(scanCard, icons.scan, "Scan")
scanHeading.Aside.Text = game.Name
local scanHint = hintTemplate:Clone()
scanHint.Text = "Sends this place's asset ids to the app. Pick where they go in the Migrate tab, the new ids get swapped in here when it's done."
scanHint.Parent = scanCard
local seg = Instance.new("Frame")
seg.Size = UDim2.new(1, 0, 0, 30)
seg.LayoutOrder = 3
tone(seg, { BackgroundColor3 = "field" })
seg.Parent = scanCard
buttonCorner:Clone().Parent = seg
hairline:Clone().Parent = seg
local segPadding = Instance.new("UIPadding")
segPadding.PaddingTop = UDim.new(0, 2)
segPadding.PaddingBottom = UDim.new(0, 2)
segPadding.PaddingLeft = UDim.new(0, 2)
segPadding.PaddingRight = UDim.new(0, 2)
segPadding.Parent = seg
local segThumb = Instance.new("Frame")
segThumb.Size = UDim2.fromScale(0.5, 1)
segThumb.BorderSizePixel = 0
tone(segThumb, { BackgroundColor3 = "main" })
segThumb.Parent = seg
local thumbCorner = Instance.new("UICorner")
thumbCorner.CornerRadius = UDim.new(0, 4)
thumbCorner.Parent = segThumb
local segButtons = {}
for index, option in { { "place", "Whole place" }, { "selection", "Selection" } } do
	local choice = Instance.new("TextButton")
	choice.BackgroundTransparency = 1
	choice.AutoButtonColor = false
	choice.Position = UDim2.fromScale((index - 1) * 0.5, 0)
	choice.Size = UDim2.fromScale(0.5, 1)
	choice.FontFace = medium
	choice.Text = option[2]
	choice.TextSize = 12
	choice.TextTruncate = Enum.TextTruncate.AtEnd
	choice.ZIndex = 2
	tone(choice, { TextColor3 = "muted" })
	choice.Parent = seg
	segButtons[option[1]] = choice
end
local scopeHint = hintTemplate:Clone()
scopeHint.TextSize = 12
scopeHint.Text = "Select something in the Explorer to scan just that part."
scopeHint.LayoutOrder = 4
tone(scopeHint, { TextColor3 = "dim" })
scopeHint.Parent = scanCard
local scanButton = mainTemplate:Clone()
setIcon(scanButton.Icon, icons.scan)
scanButton.Label.Text = "Scan place"
scanButton.LayoutOrder = 5
scanButton.Parent = scanCard
local lastScan = hintTemplate:Clone()
lastScan.TextSize = 12
lastScan.Text = ""
lastScan.Visible = false
lastScan.LayoutOrder = 7
tone(lastScan, { TextColor3 = "dim" })
lastScan.Parent = scanCard

local bulkButton = buttonTemplate:Clone()
setIcon(bulkButton.Icon, icons.move)
bulkButton.Label.Text = "Download selection"
bulkButton.LayoutOrder = 6
bulkButton.Visible = false
bulkButton.Parent = scanCard

local progressCard = cardTemplate:Clone()
progressCard.LayoutOrder = 4
progressCard.Visible = false
progressCard.Parent = root
local progressHeading = makeHeading(progressCard, icons.move, "Progress")

local track = Instance.new("Frame")
track.Size = UDim2.new(1, 0, 0, 6)
track.BorderSizePixel = 0
track.LayoutOrder = 2
tone(track, { BackgroundColor3 = "track" })
track.Parent = progressCard
round:Clone().Parent = track

local fill = Instance.new("Frame")
fill.Size = UDim2.fromScale(0, 1)
fill.BorderSizePixel = 0
tone(fill, { BackgroundColor3 = "green" })
fill.Parent = track
round:Clone().Parent = fill

local stats = Instance.new("Frame")
stats.BackgroundTransparency = 1
stats.Size = UDim2.new(1, 0, 0, 52)
stats.LayoutOrder = 3
stats.Parent = progressCard
local statsLayout = Instance.new("UIListLayout")
statsLayout.FillDirection = Enum.FillDirection.Horizontal
statsLayout.Padding = UDim.new(0, 6)
statsLayout.SortOrder = Enum.SortOrder.LayoutOrder
statsLayout.Parent = stats

local tileTemplate = Instance.new("Frame")
tileTemplate.Size = UDim2.new(1 / 3, -4, 1, 0)
tone(tileTemplate, { BackgroundColor3 = "field" })
buttonCorner:Clone().Parent = tileTemplate
local tileValue = Instance.new("TextLabel")
tileValue.Name = "Value"
tileValue.BackgroundTransparency = 1
tileValue.Position = UDim2.fromOffset(0, 7)
tileValue.Size = UDim2.new(1, 0, 0, 22)
tileValue.FontFace = bold
tileValue.Text = "0"
tileValue.TextSize = 18
tileValue.TextTruncate = Enum.TextTruncate.AtEnd
tone(tileValue, { TextColor3 = "bright" })
tileValue.Parent = tileTemplate
local tileCaption = tileValue:Clone()
tileCaption.Name = "Caption"
tileCaption.Position = UDim2.fromOffset(0, 30)
tileCaption.Size = UDim2.new(1, 0, 0, 14)
tileCaption.FontFace = medium
tileCaption.TextSize = 11
tone(tileCaption, { TextColor3 = "dim" })
tileCaption.Parent = tileTemplate

local tiles = {}
for order, name in { "done", "moved", "failed" } do
	local tile = tileTemplate:Clone()
	tile.LayoutOrder = order
	tile.Caption.Text = name
	tile.Parent = stats
	local value = Instance.new("NumberValue")
	value.Parent = tile
	value.Changed:Connect(function(n)
		tile.Value.Text = tostring(math.round(n))
	end)
	tiles[name] = value
end

local applyButton = buttonTemplate:Clone()
setIcon(applyButton.Icon, icons.refresh)
applyButton.Label.Text = "Swap ids again"
applyButton.LayoutOrder = 4
applyButton.Visible = false
applyButton.Parent = progressCard

local leftButton = buttonTemplate:Clone()
setIcon(leftButton.Icon, icons.scan)
leftButton.Label.Text = "Select leftovers"
leftButton.LayoutOrder = 5
leftButton.Visible = false
leftButton.Parent = progressCard

local note = Instance.new("Frame")
note.Size = UDim2.fromScale(1, 0)
note.AutomaticSize = Enum.AutomaticSize.Y
note.LayoutOrder = 5
note.Visible = false
note.Parent = root
buttonCorner:Clone().Parent = note
local notePadding = Instance.new("UIPadding")
notePadding.PaddingTop = UDim.new(0, 9)
notePadding.PaddingBottom = UDim.new(0, 9)
notePadding.PaddingLeft = UDim.new(0, 12)
notePadding.PaddingRight = UDim.new(0, 12)
notePadding.Parent = note
local noteIcon = iconTemplate:Clone()
noteIcon.Position = UDim2.fromOffset(0, 1)
noteIcon.Parent = note
local noteText = hintTemplate:Clone()
noteText.Position = UDim2.fromOffset(24, 0)
noteText.Size = UDim2.new(1, -24, 0, 0)
noteText.FontFace = medium
noteText.Parent = note

local footer = Instance.new("TextLabel")
footer.BackgroundTransparency = 1
footer.Size = UDim2.fromScale(1, 0)
footer.AutomaticSize = Enum.AutomaticSize.Y
footer.FontFace = medium
footer.TextSize = 11
footer.TextWrapped = true
footer.TextXAlignment = Enum.TextXAlignment.Left
footer.LayoutOrder = 6
tone(footer, { TextColor3 = "dim" })
footer.Parent = root

local hoverable = { connectButton, scanButton, bulkButton, applyButton, leftButton, disconnectButton }
for _, button in hoverable do
	button.MouseEnter:Connect(function()
		local base = button:GetAttribute("ToneBackgroundColor3")
		TweenService:Create(button, quick, { BackgroundColor3 = palette[base .. "Hover"] }):Play()
		if button == disconnectButton then
			TweenService:Create(button.Label, quick, { TextColor3 = palette.errText }):Play()
			TweenService:Create(button.Icon, quick, { ImageColor3 = palette.errText }):Play()
		end
	end)
	button.MouseLeave:Connect(function()
		local base = button:GetAttribute("ToneBackgroundColor3")
		TweenService:Create(button, quick, { BackgroundColor3 = palette[base] }):Play()
		TweenService:Create(button.UIScale, quick, { Scale = 1 }):Play()
		if button == disconnectButton then
			TweenService:Create(button.Label, quick, { TextColor3 = palette.muted }):Play()
			TweenService:Create(button.Icon, quick, { ImageColor3 = palette.muted }):Play()
		end
	end)
	button.MouseButton1Down:Connect(function()
		TweenService:Create(button.UIScale, quick, { Scale = 0.97 }):Play()
	end)
	button.MouseButton1Up:Connect(function()
		TweenService:Create(button.UIScale, quick, { Scale = 1 }):Play()
	end)
end
codeBox.Focused:Connect(function()
	TweenService:Create(codeStroke, quick, { Transparency = 0.7 }):Play()
end)
codeBox.FocusLost:Connect(function()
	TweenService:Create(codeStroke, quick, { Transparency = 0.91 }):Play()
end)
codeBox:GetPropertyChangedSignal("Text"):Connect(function()
	local digits = string.sub(string.gsub(codeBox.Text, "%D", ""), 1, 6)
	if digits ~= codeBox.Text then
		codeBox.Text = digits
	end
end)

local function fitWidth()
	local narrow = root.AbsoluteSize.X < 290
	header.Size = UDim2.new(1, 0, 0, narrow and 62 or 30)
	pill.AnchorPoint = narrow and Vector2.new(0, 0) or Vector2.new(1, 0.5)
	pill.Position = narrow and UDim2.fromOffset(0, 34) or UDim2.fromScale(1, 0.5)
	pairedText.Visible = root.AbsoluteSize.X >= 260
end
root:GetPropertyChangedSignal("AbsoluteSize"):Connect(fitWidth)
fitWidth()

local pulse = TweenService:Create(
	ring,
	TweenInfo.new(1.4, Enum.EasingStyle.Quad, Enum.EasingDirection.Out, -1),
	{ Size = UDim2.fromOffset(20, 20), BackgroundTransparency = 1 }
)

local function setPill(label, key, live)
	pillText.Text = label
	dot:SetAttribute("ToneBackgroundColor3", key)
	TweenService:Create(dot, quick, { BackgroundColor3 = palette[key] }):Play()
	if live then
		if pulse.PlaybackState ~= Enum.PlaybackState.Playing then
			ring.Size = UDim2.fromOffset(8, 8)
			ring.BackgroundTransparency = 0.4
			pulse:Play()
		end
	else
		pulse:Cancel()
		ring.BackgroundTransparency = 1
	end
end

local function paintNote()
	local icon = icons.radio
	local background = palette.line
	local shade = 0.96
	local tint = palette.muted
	if noteKind == "ok" then
		icon = icons.check
		background = palette.green
		shade = 0.9
		tint = palette.okText
	elseif noteKind == "err" then
		icon = icons.alert
		background = palette.red
		shade = 0.88
		tint = palette.errText
	end
	setIcon(noteIcon, icon)
	noteText.TextColor3 = tint
	noteIcon.ImageColor3 = tint
	note.BackgroundColor3 = background
	return shade
end

local function setStatus(message, kind)
	if message == nil or message == "" then
		noteMessage = nil
		note.Visible = false
		return
	end
	local fresh = not note.Visible or noteMessage ~= message
	noteMessage = message
	noteKind = kind
	noteText.Text = message
	local shade = paintNote()
	note.Visible = true
	if fresh then
		note.BackgroundTransparency = 1
		noteText.TextTransparency = 1
		noteIcon.ImageTransparency = 1
		TweenService:Create(note, smooth, { BackgroundTransparency = shade }):Play()
		TweenService:Create(noteText, smooth, { TextTransparency = 0 }):Play()
		TweenService:Create(noteIcon, smooth, { ImageTransparency = 0 }):Play()
	else
		note.BackgroundTransparency = shade
	end
end

local function applyTheme()
	local background = settings().Studio.Theme:GetColor(Enum.StudioStyleGuideColor.MainBackground)
	palette = background.R + background.G + background.B > 1.5 and palettes.light or palettes.dark
	for _, inst in widget:GetDescendants() do
		for _, prop in toned do
			local key = inst:GetAttribute("Tone" .. prop)
			if key then
				inst[prop] = palette[key]
			end
		end
	end
	title.Text = 'rbx<font color="' .. palette.accent .. '">yoinker</font>'
	if note.Visible then
		note.BackgroundTransparency = paintNote()
	end
end
settings().Studio.ThemeChanged:Connect(applyTheme)
applyTheme()

local spinners = {}

local function setBusy(button, label)
	if spinners[button] then
		spinners[button]:Cancel()
		spinners[button] = nil
	end
	local icon = button.Icon
	icon.Rotation = 0
	if label then
		icon:SetAttribute("Idle", icon:GetAttribute("Idle") or button.Label.Text)
		icon:SetAttribute("IdleX", icon:GetAttribute("IdleX") or icon.ImageRectOffset.X)
		icon:SetAttribute("IdleY", icon:GetAttribute("IdleY") or icon.ImageRectOffset.Y)
		icon:SetAttribute("IdleImage", icon:GetAttribute("IdleImage") or icon.Image)
		setIcon(icon, icons.loader)
		button.Label.Text = label
		spinners[button] = TweenService:Create(icon, TweenInfo.new(0.8, Enum.EasingStyle.Linear, Enum.EasingDirection.In, -1), { Rotation = 360 })
		spinners[button]:Play()
	elseif icon:GetAttribute("Idle") then
		button.Label.Text = icon:GetAttribute("Idle")
		icon.Image = icon:GetAttribute("IdleImage")
		icon.ImageRectOffset = Vector2.new(icon:GetAttribute("IdleX"), icon:GetAttribute("IdleY"))
	end
end

local function showScope()
	local has = #Selection:Get() > 0
	if scope == "selection" and not has then
		scope = "place"
	end
	TweenService:Create(segThumb, quick, { Position = UDim2.fromScale(scope == "place" and 0 or 0.5, 0) }):Play()
	tone(segButtons.place, { TextColor3 = scope == "place" and "bright" or "muted" })
	tone(segButtons.selection, { TextColor3 = scope == "selection" and "bright" or (has and "muted" or "placeholder") })
	scopeHint.Visible = not has
	bulkButton.Visible = has
	if not spinners[scanButton] then
		scanButton.Label.Text = scope == "place" and "Scan place" or "Scan selection"
	end
end

local function showVersion()
	if appVersion and pluginVersion ~= "dev" and appVersion ~= pluginVersion then
		footer.Text = "Plugin v" .. pluginVersion .. " is out of date, update it in the app"
		tone(footer, { TextColor3 = "errText" })
	else
		footer.Text = "plugin v" .. pluginVersion .. (appVersion and ("  ·  app v" .. appVersion) or "")
		tone(footer, { TextColor3 = "dim" })
	end
end

local function showView()
	local paired = token ~= ""
	pairBox.Visible = not paired
	pairedRow.Visible = paired
	scanCard.Visible = paired
	applyButton.Visible = paired and next(lastMap) ~= nil
	if not paired then
		setPill("Not connected", "dim", false)
		progressCard.Visible = false
		fill.Size = UDim2.fromScale(0, 1)
	end
	showVersion()
end

local function forget()
	token = ""
	plugin:SetSetting("rbxyoinkerToken", "")
	showView()
end

local function call(method, path, body, quiet)
	local ok, res = pcall(HttpService.RequestAsync, HttpService, {
		Url = serverUrl .. path,
		Method = method,
		Headers = { ["Content-Type"] = "application/json", ["X-Yoinker-Token"] = token },
		Body = body and HttpService:JSONEncode(body) or nil,
	})
	if not ok then
		setPill("App not running", "red", false)
		if not quiet then
			setStatus("Can't reach rbxyoinker. Start the app and try again.", "err")
		end
		return nil
	end
	local decodedOk, decoded = pcall(HttpService.JSONDecode, HttpService, res.Body)
	if res.StatusCode == 401 then
		forget()
		setStatus("This Studio was disconnected from rbxyoinker. Get a new code in the app's Migrate tab to connect again.", "err")
		return nil
	end
	if not decodedOk or not decoded.ok then
		if not quiet then
			setStatus(decodedOk and tostring(decoded.error) or "rbxyoinker sent back something odd.", "err")
		end
		return nil
	end
	if token ~= "" then
		setPill("Connected", "green", true)
	end
	if typeof(decoded.data) == "table" and decoded.data.version then
		appVersion = decoded.data.version
		showVersion()
	end
	return decoded.data
end

local function scan(config, roots)
	local foundRefs = {}
	local foundScripts = {}
	local ids = {}
	local seen = {}
	local inProps = {}
	local inScripts = {}
	local picked = {}
	local listed = {}
	for _, name in config.services do
		picked[name] = true
	end
	for _, name in config.all do
		listed[name] = true
	end
	local everything = {}
	local services = game:GetChildren()
	if roots then
		services = {}
		for _, chosen in roots do
			table.insert(everything, chosen)
			local ok, list = pcall(chosen.GetDescendants, chosen)
			if ok then
				table.move(list, 1, #list, #everything + 1, everything)
			end
		end
	end
	for _, service in services do
		local include
		if service.ClassName == "CoreGui" then
			include = not config.skip_coregui
		elseif listed[service.ClassName] then
			include = picked[service.ClassName]
		else
			include = picked.Other
		end
		if include then
			local ok, list = pcall(service.GetDescendants, service)
			if ok then
				table.move(list, 1, #list, #everything + 1, everything)
			end
		end
	end
	for index, inst in everything do
		if index % 4000 == 0 then
			task.wait()
		end
		local props = watched[inst.ClassName]
		if props then
			for _, prop in props do
				local ok, value = pcall(function()
					return inst[prop]
				end)
				local digits = ok and typeof(value) == "string" and string.match(value, "%d%d%d+")
				if digits then
					table.insert(foundRefs, { instance = inst, prop = prop, id = digits })
					inProps[digits] = true
					if not seen[digits] then
						seen[digits] = true
						table.insert(ids, digits)
					end
				end
			end
		elseif config.scripts and inst:IsA("LuaSourceContainer") then
			local ok, source = pcall(function()
				return inst.Source
			end)
			if ok and typeof(source) == "string" then
				local found = false
				for _, line in string.split(source, "\n") do
					for digits in string.gmatch(line, "%f[%w](%d+)%f[%W]") do
						if #digits >= 6 and #digits <= 16 then
							found = true
							if not inScripts[digits] then
								local trimmed = string.match(line, "^%s*(.-)%s*$")
								inScripts[digits] = { script = inst:GetFullName(), line = string.sub(trimmed, 1, 160) }
							end
							if not seen[digits] then
								seen[digits] = true
								table.insert(ids, digits)
							end
						end
					end
				end
				if found then
					table.insert(foundScripts, inst)
				end
			end
		end
	end
	local fromScripts = {}
	for digits, where in inScripts do
		if not inProps[digits] then
			table.insert(fromScripts, { id = digits, script = where.script, line = where.line })
		end
	end
	return { ids = ids, fromScripts = fromScripts, refs = foundRefs, scriptRefs = foundScripts }
end

local function usersOf(wanted)
	local found = {}
	local index = 0
	for _, service in game:GetChildren() do
		if service.ClassName ~= "CoreGui" then
			local ok, list = pcall(service.GetDescendants, service)
			for _, inst in ok and list or {} do
				index += 1
				if index % 4000 == 0 then
					task.wait()
				end
				local props = watched[inst.ClassName]
				if props then
					for _, prop in props do
						local read, value = pcall(function()
							return inst[prop]
						end)
						local digits = read and typeof(value) == "string" and string.match(value, "%d%d%d+")
						if digits and wanted[digits] then
							table.insert(found, inst)
							break
						end
					end
				elseif inst:IsA("LuaSourceContainer") then
					local read, source = pcall(function()
						return inst.Source
					end)
					if read and typeof(source) == "string" then
						for digits in string.gmatch(source, "%f[%w](%d+)%f[%W]") do
							if wanted[digits] then
								table.insert(found, inst)
								break
							end
						end
					end
				end
			end
		end
	end
	return found
end

local function selectIds(ids, label)
	local wanted = {}
	for _, id in ids do
		wanted[tostring(id)] = true
	end
	local found = usersOf(wanted)
	Selection:Set(found)
	if #found == 0 then
		setStatus("Nothing in this place uses " .. label .. " anymore.")
	else
		setStatus("Selected " .. #found .. (#found == 1 and " instance" or " instances") .. " using " .. label .. ".", "ok")
	end
end

local function insertModels(list)
	local inserted = {}
	local failed = 0
	local camera = workspace.CurrentCamera
	local spot = camera and (camera.CFrame * CFrame.new(0, 0, -20)).Position or Vector3.zero
	local recording = ChangeHistoryService:TryBeginRecording("rbxyoinker insert")
	for _, ticket in list do
		local ok, res = pcall(HttpService.RequestAsync, HttpService, {
			Url = serverUrl .. "/api/studio/file?n=" .. tostring(ticket.n),
			Method = "GET",
			Headers = { ["X-Yoinker-Token"] = token },
		})
		local items = nil
		if ok and res.StatusCode == 200 then
			local loaded, result = pcall(SerializationService.DeserializeInstancesAsync, SerializationService, buffer.fromstring(res.Body))
			items = loaded and result or nil
		end
		if typeof(items) == "table" and #items > 0 then
			for _, item in items do
				item.Parent = workspace
				if item:IsA("PVInstance") then
					pcall(item.PivotTo, item, CFrame.new(spot))
				end
				table.insert(inserted, item)
			end
		else
			failed += 1
		end
	end
	if recording then
		ChangeHistoryService:FinishRecording(recording, Enum.FinishRecordingOperation.Commit)
	end
	if #inserted > 0 then
		Selection:Set(inserted)
	end
	if failed > 0 then
		setStatus("Couldn't load " .. failed .. (failed == 1 and " model" or " models") .. " from rbxyoinker.", "err")
	elseif #list == 1 then
		setStatus("Inserted " .. tostring(list[1].name) .. ". Ctrl+Z undoes it.", "ok")
	else
		setStatus("Inserted " .. #list .. " models. Ctrl+Z undoes it.", "ok")
	end
end

local function applySwaps()
	local swapped = 0
	local waiting = 0
	local meshParts = {}
	setBusy(applyButton, "Swapping ...")
	local recording = ChangeHistoryService:TryBeginRecording("rbxyoinker migrate")
	for _, ref in refs do
		local newId = lastMap[ref.id]
		if newId and ref.instance.Parent then
			local content = "rbxassetid://" .. newId
			if ref.instance:IsA("MeshPart") and ref.prop == "MeshId" then
				local source = meshParts[newId]
				if not source then
					local ok, part = pcall(AssetService.CreateMeshPartAsync, AssetService, content)
					if ok then
						source = part
						meshParts[newId] = part
					end
				end
				if source and pcall(ref.instance.ApplyMesh, ref.instance, source) then
					ref.id = newId
					swapped += 1
				else
					waiting += 1
				end
			elseif pcall(function()
				ref.instance[ref.prop] = content
			end) then
				ref.id = newId
				swapped += 1
			end
		end
	end
	for _, container in scriptRefs do
		if container.Parent then
			pcall(ScriptEditorService.UpdateSourceAsync, ScriptEditorService, container, function(source)
				for oldId, newId in lastMap do
					local replaced, count = string.gsub(source, "%f[%w]" .. oldId .. "%f[%W]", newId)
					source = replaced
					swapped += count
				end
				return source
			end)
		end
	end
	if recording then
		ChangeHistoryService:FinishRecording(recording, Enum.FinishRecordingOperation.Commit)
	end
	setBusy(applyButton, nil)
	if waiting > 0 then
		setStatus(swapped .. " ids swapped. " .. waiting .. " meshes aren't processed by Roblox yet, click Swap ids again in a minute.")
	else
		setStatus(swapped .. " ids swapped. Ctrl+Z undoes it.", "ok")
	end
	applyButton.Visible = true
end

local function showJob(job)
	local total = 0
	local moved = 0
	local failed = 0
	for _, item in job.items do
		if item.state ~= "found" then
			total += 1
			if item.state == "ok" then
				moved += 1
			elseif item.state == "error" then
				failed += 1
			end
		end
	end
	local done = moved + failed
	progressCard.Visible = total > 0
	TweenService:Create(fill, smooth, { Size = UDim2.fromScale(total > 0 and done / total or 0, 1) }):Play()
	TweenService:Create(tiles.done, smooth, { Value = done }):Play()
	TweenService:Create(tiles.moved, smooth, { Value = moved }):Play()
	TweenService:Create(tiles.failed, smooth, { Value = failed }):Play()
	leftovers = {}
	if not job.running and typeof(job.target) == "string" and job.target ~= "" then
		for _, item in job.items do
			if item.kind ~= "" and item.state ~= "ok" and item.owner ~= job.target then
				table.insert(leftovers, tostring(item.id))
			end
		end
	end
	leftButton.Visible = #leftovers > 0
	if not spinners[leftButton] then
		leftButton.Label.Text = "Select " .. #leftovers .. (#leftovers == 1 and " leftover" or " leftovers")
	end
	progressHeading.Label.Text = job.running and "Moving" or "Progress"
	progressHeading.Aside.Text = total > 0 and (done .. " of " .. total) or ""
	if job.running then
		setStatus("Moving assets, keep Studio open.")
	elseif seenRun >= 0 and job.run_id ~= seenRun and total > 0 then
		seenRun = job.run_id
		lastMap = {}
		for _, item in job.items do
			if item.new_id then
				lastMap[tostring(item.id)] = tostring(item.new_id)
			end
		end
		if job.mode == "swap" then
			applySwaps()
		else
			setStatus("Done. Copy the mapping from the app.", "ok")
		end
	end
end

local function connect()
	if busy then
		return
	end
	local code = string.gsub(codeBox.Text, "%D", "")
	if #code ~= 6 then
		setStatus("The code is 6 digits.", "err")
		return
	end
	busy = true
	setBusy(connectButton, "Connecting ...")
	local paired = call("POST", "/api/studio/pair", { code = code })
	setBusy(connectButton, nil)
	busy = false
	if not paired then
		return
	end
	token = paired.token
	plugin:SetSetting("rbxyoinkerToken", token)
	codeBox.Text = ""
	showView()
	setPill("Connected", "green", true)
	setStatus("Connected. Scan the place when you're ready.", "ok")
end

local function scanAndSend()
	if busy then
		return
	end
	busy = true
	setBusy(scanButton, "Scanning ...")
	setStatus("")
	local config = call("GET", "/api/studio/scan")
	local found = config and scan(config, scope == "selection" and Selection:Get() or nil)
		or { ids = {}, fromScripts = {}, refs = {}, scriptRefs = {} }
	refs = found.refs
	scriptRefs = found.scriptRefs
	local ids = found.ids
	local fromScripts = found.fromScripts
	local sent = nil
	if config and #ids == 0 then
		setStatus(scope == "selection" and "No asset ids found in the selection." or "No asset ids found in the parts of the place you picked to scan.")
	elseif config then
		sent = call("POST", "/api/studio/submit", { place = game.Name, place_id = game.PlaceId, ids = ids, from_scripts = fromScripts })
	end
	setBusy(scanButton, nil)
	busy = false
	if not sent then
		return
	end
	seenRun = sent.run_id
	lastMap = {}
	applyButton.Visible = false
	fill.Size = UDim2.fromScale(0, 1)
	lastScan.Text = "Last scan found " .. sent.count .. " ids at " .. os.date("%H:%M")
	lastScan.Visible = true
	setStatus("Sent " .. sent.count .. " ids. Pick where they go in rbxyoinker's Migrate tab.")
end

connectButton.MouseButton1Click:Connect(connect)
scanButton.MouseButton1Click:Connect(scanAndSend)
applyButton.MouseButton1Click:Connect(applySwaps)

bulkButton.MouseButton1Click:Connect(function()
	if busy or #Selection:Get() == 0 then
		return
	end
	busy = true
	setBusy(bulkButton, "Reading ...")
	local config = call("GET", "/api/studio/scan")
	local found = config and scan(config, Selection:Get())
	local sent = nil
	if found and #found.ids == 0 then
		setStatus("No asset ids found in the selection.")
	elseif found then
		sent = call("POST", "/api/studio/bulk", { ids = found.ids })
	end
	setBusy(bulkButton, nil)
	busy = false
	if sent then
		setStatus("Sent " .. sent.count .. " ids to rbxyoinker's Bulk tab.", "ok")
	end
end)

leftButton.MouseButton1Click:Connect(function()
	if busy or #leftovers == 0 then
		return
	end
	busy = true
	setBusy(leftButton, "Selecting ...")
	selectIds(leftovers, #leftovers == 1 and "the old id" or (#leftovers .. " old ids"))
	setBusy(leftButton, nil)
	busy = false
end)

segButtons.place.MouseButton1Click:Connect(function()
	scope = "place"
	showScope()
end)
segButtons.selection.MouseButton1Click:Connect(function()
	if #Selection:Get() > 0 then
		scope = "selection"
	end
	showScope()
end)
Selection.SelectionChanged:Connect(showScope)

codeBox.FocusLost:Connect(function(enterPressed)
	if enterPressed then
		connect()
	end
end)

disconnectButton.MouseButton1Click:Connect(function()
	call("POST", "/api/studio/unpair", {}, true)
	forget()
	setStatus("Disconnected. Get a new code in the app to connect again.")
end)

toggle.Click:Connect(function()
	widget.Enabled = not widget.Enabled
end)

showView()
showScope()
if token == "" then
	setStatus("The rbxyoinker app has to be running.")
end

task.spawn(function()
	while true do
		if token ~= "" then
			if widget.Enabled then
				local job = call("GET", "/api/studio/job", nil, true)
				if job then
					showJob(job)
					if job.show then
						selectIds(job.show, #job.show == 1 and ("#" .. job.show[1]) or (#job.show .. " ids"))
					end
					if job.insert then
						insertModels(job.insert)
					end
				end
				task.wait(2)
			else
				local pong = call("POST", "/api/studio/ping", { place = game.Name, place_id = game.PlaceId }, true)
				if pong and pong.show then
					selectIds(pong.show, #pong.show == 1 and ("#" .. pong.show[1]) or (#pong.show .. " ids"))
				end
				if pong and pong.insert then
					insertModels(pong.insert)
				end
				task.wait(5)
			end
		else
			task.wait(2)
		end
	end
end)
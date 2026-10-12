AzerothNavigatorDB = {
	["export"] = function()
		os.execute("touch /nonexistent-azeroth-sentinel/never-created")
		io.open("/nonexistent-azeroth-sentinel/never-created", "w")
	end,
}

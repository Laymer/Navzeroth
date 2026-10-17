-- Synthetic RXP-format route for tests only (task V2.3). Hand-written.
-- Every zone, node, quest, spell and item name or number here is invented; none
-- is game data. Paired with synthetic-facts.json. Read as data, never executed.

RXPGuides.RegisterGuide([[
#forever
<< Human
#name 1-3 Test Vale
#displayname 1-3 Test Vale
#version 1
#group Synthetic Route
#subgroup Synthetic
#defaultfor Human Warrior
#next 3-5 Sample Hills

step
    .goto Test Vale,50.00,50.00
    .accept 900001 >>Accept First Steps

step
    #sticky
    #label hunt
    .goto Test Vale,40.5,60.25,30,0
    .mob Practice Dummy
    .complete 900001,1 --Dummies
    .complete 900001,2
    +Hit the dummies

step
    #completewith hunt
    .waypoint Test Vale,41.00,61.00
    *Watch the path

step
    .goto Test Vale,50.00,50.00
    .turnin 900001,2 >>Turn in First Steps
    .accept 900002 >>Accept Second Steps
    .accept 900003 >>Accept Drop-in

step
    .goto Test Vale,51,51
    .train 100 >>Train a warrior spell << Warrior
    .train 200 >>Train a mage spell << Mage
    .trainer >>Talk to the trainer

step << Mage
    .goto Test Vale,52,52
    .accept 900099 >>A mage-only quest

step
    .goto Test Vale,45,45
    .xp 2 >>Reach level 2
    .xp 2.5
    .xp 2+100
    .xp 2-50
    .xp 3,1
    .xp <9,1
    .maxlevel 10

step
    -- a comment line inside a step
    .goto Test Vale,46,46
    .abandon 900003 >>Abandon Drop-in
    .collect 700001,4 >>Collect four pebbles
    .use 700001
    .vendor >>Sell junk
    .home >>Set your hearthstone

step
    #optional
    .goto Test Vale,47,47
    .isQuestAvailable 900050
    .accept 900050 >>Optional pickup
]])

RXPGuides.RegisterGuide([[
#forever
<< Human
#name 3-5 Sample Hills
#version 1
#group Synthetic Route
#defaultfor Human Warrior

step
    .fp Testford >>Get the Testford flight path
    .fly Testford >>Fly to Testford

step
    .zone Sample Hills >>Enter Sample Hills
    .gossipoption 12345 >>Pick the first option

step
    .goto Sample Hills,20,20
    .complete 900002,1
    .turnin 900002 >>Turn in Second Steps

step
    #requires hunt
    #level 3
    .goto Sample Hills,21,21
    .isQuestTurnedIn 900002
    .accept 900004 >>Accept Third Steps

step
    .goto Sample Hills,22,22
    .isOnQuest 900004
    .isQuestComplete 900004
    .turnin -900004 >>Turn in Third Steps
    .accept -900005 >>Only if already on it

step
    .hs >>Hearth
    .deathskip >>Die and respawn
]])

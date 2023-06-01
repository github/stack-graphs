command = 1
current_room = 2
character = 3

match command.split():
    # ^ defined: 1
    case ["quit"]:
        print("Goodbye!")
        quit_game()
    case ["look"]:
        current_room.describe()
        # ^ defined: 2
    case ["get", obj]:
        character.get(obj, current_room)
        # ^ defined: 3
        #              ^ defined: 13
        #                    ^ defined: 2
    case ["go", direction]:
        current_room = current_room.neighbor(direction)
        #                                      ^ defined: 18
    case { "foo": foo }:
        print(foo)
        #      ^ defined: 21
    case {bar,quux}:
        print(bar,quux)
        #      ^ defined: 24
        #           ^ defined: 24
    case ["grab", { "key": {garply}}]:
        print(garply)
        #       ^ defined: 28
    case ["drop", *objs]:
        print(objs)
        #      ^ defined: 31
    case ["get", obj] | ["pick", "up", obj] | ["pick", obj, "up"]:
        print(obj)
        #      ^ defined: 34, 34, 34
    case ["go", ("north" | "south" | "east" | "west") as direction2]:
        current_room = current_room.neighbor(direction2)
        #                                       ^ defined: 37

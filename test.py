
def make_test(func):
    import sys
    import json
    program = []
    for l in sys.stdin:
        program.append(l)
    ret = json.loads("\n".join(program).strip())
    return ret["code"] + f"\nresult = {func}(*args)" , eval(ret["test_case"])

# TODO: It's unlikely that someone would make a function called `make_test`,
# but probably want to not include `test` in global vars
def test(program, test_case):
    import sys
    for args, expect in test_case:
        # Use context with only arguements for locals
        # Also copies global context so it is not modified
        local = {"args": args}
        exec(program, globals().copy(), local)
        if local["result"] != expect:
            print(f"Test case failed on input `{args}`: Expected \n`{expect}`\nbut got \n`{local['result']}`", end='')
            sys.exit(2)

    print("All test cases passed!", end='')


if __name__ == "__main__":
    from argparse import ArgumentParser
    parser = ArgumentParser()
    parser.add_argument("func", type=str)
    args = parser.parse_args()
    program, test_case = make_test(args.func)
    test(program, test_case)

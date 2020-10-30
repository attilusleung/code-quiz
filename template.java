@SuppressWarnings("unchecked")
public class Main {

    static class TestCase<I, O> {
        public I input;
        public O output;

        public TestCase(I input, O output) {
            this.input = input;
            this.output = output;
        }

    }

    static {{test_case}};

    public static void main(String[] args) {

        for (var t : testCase) {
            Solution s = new Solution();
            var output = s.{{func_call}};
            if (!t.output.equals(output)) {
                System.out.printf("Test case failed on input %s: Expected\n%s\nbut got\n%s\n", java.util.Arrays.toString(t.input), t.output, output);
                System.exit(2);
            }
        }
        System.out.println("All test cases passed!");
    }
}


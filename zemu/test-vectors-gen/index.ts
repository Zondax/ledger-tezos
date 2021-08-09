(async () => {
    const myArgs = process.argv.slice(2);

    const options: {[index: string]: string} = {
        // "simple": "./simple.ts",
        "legacy": "./legacy.ts"
    };

    if (myArgs[0] === undefined) {
        myArgs[0] = "legacy"
    }

    const select = options[myArgs[0]]

    const filename = require('path').resolve(myArgs[1] ? myArgs[1] : `test-vectors/${myArgs[0]}.json`);

    const iters = myArgs[2] ? myArgs[2] : 10;

    if (select === undefined) {
        console.log(`Invalid option ${select} specified.`);
        console.log(`Available options: ${Object.keys(options)}`);
    } else {
        console.log(`Generating ${iters} test vectors via ${myArgs[0]} and saving to ${filename}`)
        const vectors = await require(select).run(iters);

        require('fs').writeFileSync(filename, JSON.stringify(vectors, null, 4));
        console.log(`Saved ${iters} test vectors to ${filename}`);
    }
})();

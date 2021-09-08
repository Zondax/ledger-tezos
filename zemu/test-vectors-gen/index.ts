(async () => {
    let myArgs = process.argv.slice(2);

    const force = myArgs[0] == '-f';
    if (force) {
        myArgs = myArgs.slice(1); //chop off the -f
    }

    const options: {[index: string]: string} = {
        // "simple": "./simple.ts",
        "legacy": "./legacy.ts",
        "delegation": "./delegation.ts",
        "reveal": "./reveal.ts",
        "origination": "./origination.ts",
        "ballot": "./ballot.ts",
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
        const fs = require('fs');
        console.log(`Generating ${iters} test vectors via ${myArgs[0]} and saving to ${filename}`)

        try {
            fs.accessSync(filename);
            //file exists
            if (force) {
                console.log(`${filename} exists but -f was passed so it will be overridden`)
                throw "forced generation"
            }

            console.log(`${filename} exists, pass '-f' as first argument to override`)
        } catch(no_exist) {
            const vectors = await require(select).run(iters);

            fs.writeFileSync(filename, JSON.stringify(vectors, null, 4));
            console.log(`Saved ${iters} test vectors to ${filename}`);
        }
    }
})();

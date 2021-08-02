(async () => {
    const myArgs = process.argv.slice(2);

    const options: {[index: string]: string} = {
        "simple": "./simple.ts",
        "legacy": "./legacy.ts"
    };

    if (myArgs[0] === undefined) {
        myArgs[0] = "simple"
    }

    const select = options[myArgs[0]]

    if (select === undefined) {
        console.log(`Invalid option ${select} specified.`);
        console.log(`Available options: ${Object.keys(options)}`);
    } else {
        await require(select).run();
    }
})();

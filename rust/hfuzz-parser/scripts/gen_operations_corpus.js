var fs = require("fs");
var path = require("path");

const myArgs = process.argv.slice(2);

var input_dir = myArgs[0] ? myArgs[0] : ".";
input_dir = path.normalize(input_dir);

var output_dir = myArgs[1]
  ? myArgs[1]
  : path.join(__dirname, "../hfuzz_workspace/operations/input");
output_dir = path.normalize(output_dir);

//make sure output folder exists
fs.mkdirSync(output_dir, { recursive: true }, function (err) {
  console.log("Error creating output folder", err);
});

//read input directory
fs.readdir(input_dir, function (err, files) {
  if (err) {
    console.log("Error reading all files in input folder", err);
  }
  console.log(
    `Reading all json files in ${input_dir}. ${files.length} file(s) found.`
  );

  files
    //only get files with the json extension
    .filter((filename) => path.extname(filename) == ".json")
    .forEach(function (file) {
      //read file
      fs.readFile(path.join(input_dir, file), "utf8", function (err, data) {
        if (err) {
          console.log("Error reading file", err);
        }

        //parse as JSON array
        var data = JSON.parse(data);
        var counter = 0;

        //for each object in the array
        data.forEach(function (op) {
          //retrieve the blob (hex data)
          var data = Buffer.from(op.blob, "hex");

          //construct filename
          var filename;
          if (op.name) {
            //if object has a name, use it
            filename = op.name
              .replaceAll(" ", "_") //replace spaces with _
              .toLowerCase(); //all lowercase
          } else {
            //make a string with the counter and padding
            filename = counter.toString("16").padStart(5, 0);

            const base = path.basename(file, ".json"); //retrieve input file name
            filename = base.concat(".", filename); //use counter and input file name
            counter++;
          }
          filename = filename.concat(".bin");

          console.log(`Writing binary file to ${filename}`);

          //finally write file in binary mode
          fs.writeFile(
            path.join(output_dir, filename),
            data,
            "binary",
            function (err) {
              if (err) {
                console.log("Error writing file to output", err);
              }
            }
          );
        });
      });
    });
});
